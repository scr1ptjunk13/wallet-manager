import requests
import json
import sqlite3
import time
import random
from bs4 import BeautifulSoup
from datetime import datetime, timedelta
import logging
from urllib.parse import urljoin, urlparse
import re
from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.chrome.service import Service
from webdriver_manager.chrome import ChromeDriverManager
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.common.exceptions import TimeoutException, WebDriverException
import traceback

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class EnhancedGalxeScraper:
    def __init__(self, db_path='galxe_campaigns.db'):
        self.base_url = 'https://app.galxe.com'
        self.explore_url = 'https://app.galxe.com/quest/explore/all'
        self.db_path = db_path
        
        # Updated user agents
        self.user_agents = [
            'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
            'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
            'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
        ]
        
        self.session = requests.Session()
        self.setup_session()
        self.setup_database()
    
    def setup_session(self):
        """Configure session with headers and settings"""
        self.session.headers.update({
            'User-Agent': random.choice(self.user_agents),
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
            'Accept-Language': 'en-US,en;q=0.5',
            'Accept-Encoding': 'gzip, deflate, br',
            'Connection': 'keep-alive',
            'Upgrade-Insecure-Requests': '1',
            'Sec-Fetch-Dest': 'document',
            'Sec-Fetch-Mode': 'navigate',
            'Sec-Fetch-Site': 'none',
            'Cache-Control': 'max-age=0'
        })
        
        from requests.adapters import HTTPAdapter
        from urllib3.util.retry import Retry
        
        retry_strategy = Retry(
            total=3,
            backoff_factor=1,
            status_forcelist=[429, 500, 502, 503, 504],
        )
        adapter = HTTPAdapter(max_retries=retry_strategy)
        self.session.mount("http://", adapter)
        self.session.mount("https://", adapter)
    
    def setup_database(self):
        """Initialize SQLite database for storing campaigns"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS campaigns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                campaign_id TEXT UNIQUE,
                project_name TEXT,
                campaign_title TEXT,
                campaign_url TEXT,
                task_count INTEGER,
                task_types TEXT,
                reward_type TEXT,
                reward_details TEXT,
                deadline TEXT,
                deadline_timestamp INTEGER,
                status TEXT,
                estimated_value TEXT,
                description TEXT,
                chain TEXT,
                participants INTEGER,
                is_featured BOOLEAN DEFAULT 0,
                difficulty_level TEXT,
                scraped_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        ''')
        
        conn.commit()
        conn.close()
        logger.info("Database initialized successfully")
    
    def get_selenium_driver(self):
        """Create and configure Selenium WebDriver"""
        options = Options()
        options.add_argument('--headless')
        options.add_argument('--disable-gpu')
        options.add_argument('--no-sandbox')
        options.add_argument('--disable-dev-shm-usage')
        options.add_argument('--disable-blink-features=AutomationControlled')
        options.add_argument('--disable-extensions')
        options.add_argument('--disable-plugins')
        options.add_argument('--disable-images')
        options.add_argument(f'user-agent={random.choice(self.user_agents)}')
        options.add_argument('--window-size=1920,1080')
        
        service = Service(ChromeDriverManager().install())
        driver = webdriver.Chrome(service=service, options=options)
        driver.set_page_load_timeout(30)
        return driver
    
    def scrape_campaign_urls(self, max_scroll=15):
        """Scrape campaign URLs from explore page with better selectors"""
        logger.info("Scraping campaign URLs from explore page...")
        
        driver = None
        campaign_urls = set()
        
        try:
            driver = self.get_selenium_driver()
            driver.get(self.explore_url)
            
            # Wait for initial load
            WebDriverWait(driver, 10).until(
                EC.presence_of_element_located((By.TAG_NAME, "body"))
            )
            time.sleep(5)
            
            # Scroll to load more campaigns
            prev_height = 0
            for i in range(max_scroll):
                driver.execute_script("window.scrollTo(0, document.body.scrollHeight);")
                time.sleep(2)
                
                # Check if we've reached the bottom
                new_height = driver.execute_script("return document.body.scrollHeight")
                if i > 0 and new_height == prev_height:
                    logger.info(f"Reached bottom of page at scroll {i}")
                    break
                prev_height = new_height
                
                logger.info(f"Scrolled {i+1}/{max_scroll} times")
            
            # Parse the page
            soup = BeautifulSoup(driver.page_source, 'html.parser')
            
            # Enhanced campaign link detection
            campaign_links = []
            
            # Method 1: Find all links with href containing /quest/
            quest_links = soup.find_all('a', href=re.compile(r'/quest/[^/]+/?$'))
            for link in quest_links:
                href = link.get('href')
                if href and '/quest/' in href:
                    campaign_links.append(href)
            
            # Method 2: Find links in campaign cards
            card_selectors = [
                'div[class*="card"] a[href*="/quest/"]',
                'div[class*="campaign"] a[href*="/quest/"]',
                'div[class*="item"] a[href*="/quest/"]',
                'a[href*="/quest/"][class*="card"]'
            ]
            
            for selector in card_selectors:
                links = soup.select(selector)
                for link in links:
                    href = link.get('href')
                    if href:
                        campaign_links.append(href)
            
            # Clean and process URLs
            for href in campaign_links:
                if href.startswith('/'):
                    full_url = urljoin(self.base_url, href)
                else:
                    full_url = href
                
                # Clean URL (remove query params and fragments)
                clean_url = full_url.split('?')[0].split('#')[0]
                
                # Filter out explore page and other non-campaign pages
                if ('/quest/' in clean_url and 
                    clean_url != self.explore_url and 
                    '/quest/explore' not in clean_url):
                    campaign_urls.add(clean_url)
            
            logger.info(f"Found {len(campaign_urls)} unique campaign URLs")
            return list(campaign_urls)
            
        except Exception as e:
            logger.error(f"Error scraping campaign URLs: {e}")
            logger.error(traceback.format_exc())
            return []
        finally:
            if driver:
                driver.quit()
    
    def extract_campaign_details(self, campaign_url):
        """Extract detailed information from individual campaign page with improved selectors"""
        logger.info(f"Extracting details from: {campaign_url}")
        
        driver = None
        try:
            driver = self.get_selenium_driver()
            driver.get(campaign_url)
            
            # Wait for page to load
            WebDriverWait(driver, 15).until(
                EC.presence_of_element_located((By.TAG_NAME, "body"))
            )
            time.sleep(3)
            
            # Try to wait for specific elements to load
            try:
                WebDriverWait(driver, 10).until(
                    EC.any_of(
                        EC.presence_of_element_located((By.TAG_NAME, "h1")),
                        EC.presence_of_element_located((By.CSS_SELECTOR, "[class*='title']")),
                        EC.presence_of_element_located((By.CSS_SELECTOR, "[class*='campaign']"))
                    )
                )
            except TimeoutException:
                logger.warning("Timeout waiting for campaign content to load")
            
            soup = BeautifulSoup(driver.page_source, 'html.parser')
            
            campaign_data = {
                'campaign_url': campaign_url,
                'campaign_id': self.extract_campaign_id(campaign_url),
                'project_name': self.extract_project_name(soup, driver),
                'campaign_title': self.extract_campaign_title(soup, driver),
                'task_count': self.extract_task_count(soup, driver),
                'task_types': self.extract_task_types(soup, driver),
                'reward_type': self.extract_reward_type(soup, driver),
                'reward_details': self.extract_reward_details(soup, driver),
                'deadline': self.extract_deadline(soup, driver),
                'deadline_timestamp': self.extract_deadline_timestamp(soup, driver),
                'participants': self.extract_participants(soup, driver),
                'status': self.extract_status(soup, driver),
                'description': self.extract_description(soup, driver),
                'chain': self.extract_chain(soup, driver),
                'estimated_value': self.extract_estimated_value(soup, driver),
                'difficulty_level': self.extract_difficulty_level(soup, driver),
                'is_featured': self.extract_is_featured(soup, driver)
            }
            
            logger.info(f"Extracted: {campaign_data.get('campaign_title', 'Unknown')} - {campaign_data.get('project_name', 'Unknown')}")
            return campaign_data
            
        except Exception as e:
            logger.error(f"Error extracting campaign details from {campaign_url}: {e}")
            logger.error(traceback.format_exc())
            return None
        finally:
            if driver:
                driver.quit()
    
    def extract_campaign_id(self, url):
        """Extract campaign ID from URL"""
        try:
            # Match pattern like /quest/campaign-id or /quest/campaign-id/
            match = re.search(r'/quest/([^/?]+)', url)
            return match.group(1) if match else None
        except:
            return None
    
    def extract_project_name(self, soup, driver):
        """Extract project name with improved selectors"""
        try:
            # Try different methods to find project name
            
            # Method 1: Look for project name in breadcrumbs or header
            selectors = [
                'nav a span',  # Breadcrumb navigation
                'header span',  # Header area
                'h1 + div span',  # Below main title
                'div[class*="space"] span',  # Space name
                'div[class*="project"] span',  # Project container
                'img[alt] + span',  # Next to project logo
                'div[class*="breadcrumb"] span',  # Breadcrumb
                'a[href*="/space/"] span',  # Space link
            ]
            
            for selector in selectors:
                elements = soup.select(selector)
                for elem in elements:
                    text = elem.get_text(strip=True)
                    if text and 5 < len(text) < 50 and not text.lower() in ['quest', 'campaign', 'galxe']:
                        return text
            
            # Method 2: Look for project logo alt text
            img_elements = soup.find_all('img', alt=True)
            for img in img_elements:
                alt_text = img.get('alt', '').strip()
                if alt_text and 5 < len(alt_text) < 50 and 'logo' not in alt_text.lower():
                    return alt_text
            
            # Method 3: Extract from URL structure
            url_parts = driver.current_url.split('/')
            if len(url_parts) > 4:
                potential_project = url_parts[4]
                if potential_project and len(potential_project) > 2:
                    return potential_project.replace('-', ' ').title()
            
            return None
        except Exception as e:
            logger.error(f"Error extracting project name: {e}")
            return None
    
    def extract_campaign_title(self, soup, driver):
        """Extract campaign title with improved selectors"""
        try:
            # Try different selectors for campaign title
            selectors = [
                'h1',  # Main heading
                'h2',  # Secondary heading
                'div[class*="title"] h1',  # Title container
                'div[class*="title"] h2',  # Title container
                'div[class*="campaign"] h1',  # Campaign container
                'div[class*="campaign"] h2',  # Campaign container
                'div[class*="quest"] h1',  # Quest container
                'div[class*="quest"] h2',  # Quest container
            ]
            
            for selector in selectors:
                elem = soup.select_one(selector)
                if elem:
                    text = elem.get_text(strip=True)
                    if text and len(text) > 5:
                        return text
            
            # Try to extract from page title
            title_elem = soup.find('title')
            if title_elem:
                title_text = title_elem.get_text(strip=True)
                if title_text and 'galxe' not in title_text.lower():
                    # Clean up the title
                    title_text = title_text.replace(' | Galxe', '').replace(' - Galxe', '')
                    return title_text
            
            return None
        except Exception as e:
            logger.error(f"Error extracting campaign title: {e}")
            return None
    
    def extract_task_count(self, soup, driver):
        """Extract number of tasks with improved detection"""
        try:
            task_count = 0
            
            # Method 1: Look for task list items
            task_selectors = [
                'div[class*="task"]',  # Task containers
                'div[class*="entry"]',  # Entry containers
                'li[class*="task"]',  # Task list items
                'div[class*="requirement"]',  # Requirement items
                'div[class*="step"]',  # Step items
                'input[type="checkbox"]',  # Checkboxes for tasks
                'button[class*="task"]',  # Task buttons
            ]
            
            for selector in task_selectors:
                elements = soup.select(selector)
                if elements:
                    task_count = max(task_count, len(elements))
            
            # Method 2: Look for numbered tasks in text
            page_text = soup.get_text()
            numbered_tasks = re.findall(r'^\d+\.', page_text, re.MULTILINE)
            if numbered_tasks:
                task_count = max(task_count, len(numbered_tasks))
            
            # Method 3: Look for "X tasks" or similar patterns
            task_patterns = [
                r'(\d+)\s*tasks?',
                r'(\d+)\s*steps?',
                r'(\d+)\s*requirements?',
                r'(\d+)\s*entries?',
                r'Complete\s*(\d+)',
            ]
            
            for pattern in task_patterns:
                matches = re.findall(pattern, page_text, re.IGNORECASE)
                if matches:
                    try:
                        count = int(matches[0])
                        task_count = max(task_count, count)
                    except ValueError:
                        continue
            
            return task_count if task_count > 0 else 0
        except Exception as e:
            logger.error(f"Error extracting task count: {e}")
            return 0
    
    def extract_task_types(self, soup, driver):
        """Extract types of tasks with improved detection"""
        try:
            task_types = set()
            page_text = soup.get_text().lower()
            
            # Enhanced task type detection
            task_indicators = {
                'Twitter': ['twitter', 'tweet', 'follow', 'retweet', 'x.com', '@'],
                'Telegram': ['telegram', 'join channel', 'join group', 't.me'],
                'Discord': ['discord', 'join server', 'discord.gg'],
                'Wallet Connect': ['connect wallet', 'wallet', 'metamask', 'connect'],
                'On-chain': ['transaction', 'swap', 'stake', 'bridge', 'mint', 'deploy', 'interact'],
                'Visit': ['visit', 'website', 'page', 'browse'],
                'Email': ['email', 'subscribe', 'newsletter', 'signup'],
                'Quiz': ['quiz', 'question', 'answer', 'test'],
                'Referral': ['referral', 'invite', 'refer', 'share'],
                'GitHub': ['github', 'star', 'fork', 'repository'],
                'YouTube': ['youtube', 'subscribe', 'watch', 'like video'],
                'Medium': ['medium', 'clap', 'follow on medium'],
                'Like': ['like', 'heart', 'thumbs up'],
                'Comment': ['comment', 'reply', 'discuss']
            }
            
            # Check for task indicators in page text
            for task_type, keywords in task_indicators.items():
                for keyword in keywords:
                    if keyword in page_text:
                        task_types.add(task_type)
                        break
            
            # Also check for social media domains
            social_domains = {
                'Twitter': ['twitter.com', 'x.com'],
                'Telegram': ['t.me'],
                'Discord': ['discord.gg', 'discord.com'],
                'GitHub': ['github.com'],
                'YouTube': ['youtube.com', 'youtu.be'],
                'Medium': ['medium.com'],
                'LinkedIn': ['linkedin.com'],
                'Instagram': ['instagram.com']
            }
            
            for task_type, domains in social_domains.items():
                for domain in domains:
                    if domain in page_text:
                        task_types.add(task_type)
                        break
            
            return ', '.join(sorted(task_types)) if task_types else 'Unknown'
        except Exception as e:
            logger.error(f"Error extracting task types: {e}")
            return 'Unknown'
    
    def extract_reward_type(self, soup, driver):
        """Extract reward type with improved detection"""
        try:
            page_text = soup.get_text().lower()
            detected_types = set()
            
            # Enhanced reward type detection
            reward_indicators = {
                'NFT': ['nft', 'non-fungible', 'collectible', 'digital art'],
                'Tokens': ['tokens', 'usdt', 'usdc', 'eth', 'bnb', 'busd', 'dai'],
                'Points': ['points', 'xp', 'experience', 'score'],
                'OAT': ['oat', 'achievement', 'badge', 'credential'],
                'Whitelist': ['whitelist', 'allowlist', 'early access'],
                'Airdrop': ['airdrop', 'drop', 'claim'],
                'Prize': ['prize', 'reward pool', 'prize pool'],
                'Lottery': ['lottery', 'raffle', 'draw', 'lucky draw']
            }
            
            for reward_type, keywords in reward_indicators.items():
                for keyword in keywords:
                    if keyword in page_text:
                        detected_types.add(reward_type)
                        break
            
            # Look for currency symbols and amounts
            currency_pattern = r'(\$|€|£|¥)?\s*(\d+(?:,\d+)*(?:\.\d+)?)\s*(USD|USDT|USDC|ETH|BNB|BTC|SOL|MATIC|AVAX|DOT|ADA|LINK|UNI|AAVE|COMP|MKR|SNX|YFI|SUSHI|CRV|BAL|ALPHA|CAKE|tokens?)'
            if re.search(currency_pattern, page_text, re.IGNORECASE):
                detected_types.add('Tokens')
            
            return ', '.join(sorted(detected_types)) if detected_types else 'Unknown'
        except Exception as e:
            logger.error(f"Error extracting reward type: {e}")
            return 'Unknown'
    
    def extract_reward_details(self, soup, driver):
        """Extract detailed reward information"""
        try:
            page_text = soup.get_text()
            reward_details = []
            
            # Enhanced reward amount detection
            reward_patterns = [
                r'(\d+(?:,\d+)*(?:\.\d+)?)\s*(USDT|USD|USDC|ETH|BNB|BTC|SOL|MATIC|AVAX|DOT|ADA|LINK|UNI|AAVE|COMP|MKR|SNX|YFI|SUSHI|CRV|BAL|ALPHA|CAKE)',
                r'(\d+(?:,\d+)*)\s*(NFTs?|tokens?|points?|OATs?)',
                r'Total\s*(?:Rewards?|Prize)?\s*:?\s*(\d+(?:,\d+)*(?:\.\d+)?)\s*([A-Z]{3,})',
                r'Pool\s*:?\s*(\d+(?:,\d+)*(?:\.\d+)?)\s*([A-Z]{3,})',
                r'(\d+(?:,\d+)*(?:\.\d+)?)\s*([A-Z]{3,})\s*(?:rewards?|prize|pool)',
                r'Up\s*to\s*(\d+(?:,\d+)*(?:\.\d+)?)\s*([A-Z]{3,})',
                r'Win\s*(\d+(?:,\d+)*(?:\.\d+)?)\s*([A-Z]{3,})'
            ]
            
            for pattern in reward_patterns:
                matches = re.findall(pattern, page_text, re.IGNORECASE)
                for match in matches:
                    if isinstance(match, tuple) and len(match) == 2:
                        amount, currency = match
                        reward_details.append(f"{amount} {currency}")
            
            # Look for specific reward descriptions
            reward_descriptions = []
            desc_patterns = [
                r'Reward\s*:?\s*([^.\n]+)',
                r'Prize\s*:?\s*([^.\n]+)',
                r'Win\s*:?\s*([^.\n]+)',
                r'Get\s*:?\s*([^.\n]+)',
                r'Earn\s*:?\s*([^.\n]+)'
            ]
            
            for pattern in desc_patterns:
                matches = re.findall(pattern, page_text, re.IGNORECASE)
                for match in matches:
                    if len(match.strip()) > 5 and len(match.strip()) < 100:
                        reward_descriptions.append(match.strip())
            
            # Combine all reward information
            all_rewards = reward_details + reward_descriptions[:2]  # Limit descriptions
            
            return ' + '.join(all_rewards[:5]) if all_rewards else None  # Limit to 5 items
        except Exception as e:
            logger.error(f"Error extracting reward details: {e}")
            return None
    
    def extract_deadline(self, soup, driver):
        """Extract campaign deadline with improved detection"""
        try:
            page_text = soup.get_text()
            
            # Enhanced deadline patterns
            deadline_patterns = [
                r'(?:End[s]?|Deadline|Expires?|Until|Closes?)\s*(?:at|on|:)?\s*(\d{4}[-/]\d{1,2}[-/]\d{1,2}(?:\s+\d{1,2}:\d{2})?)',
                r'(?:End[s]?|Deadline|Expires?|Until|Closes?)\s*(?:at|on|:)?\s*(\d{1,2}[-/]\d{1,2}[-/]\d{4}(?:\s+\d{1,2}:\d{2})?)',
                r'(\d{4}[-/]\d{1,2}[-/]\d{1,2}\s+\d{1,2}:\d{2}(?::\d{2})?)',
                r'(\d{1,2}[-/]\d{1,2}[-/]\d{4}\s+\d{1,2}:\d{2}(?::\d{2})?)',
                r'(\d{4}[-/]\d{1,2}[-/]\d{1,2})',
                r'(\d{1,2}[-/]\d{1,2}[-/]\d{4})',
                r'(\d{1,2}\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{4})',
                r'((?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2},?\s+\d{4})',
                r'(\d{1,2}\s+days?\s+left)',
                r'(\d{1,2}\s+hours?\s+left)',
                r'(Ends?\s+in\s+\d+\s+(?:days?|hours?|minutes?))'
            ]
            
            for pattern in deadline_patterns:
                matches = re.findall(pattern, page_text, re.IGNORECASE)
                if matches:
                    return matches[0].strip()
            
            return None
        except Exception as e:
            logger.error(f"Error extracting deadline: {e}")
            return None
    
    def extract_deadline_timestamp(self, soup, driver):
        """Extract deadline as timestamp"""
        try:
            deadline_str = self.extract_deadline(soup, driver)
            if not deadline_str:
                return None
            
            # Try to parse various date formats
            date_formats = [
                '%Y-%m-%d %H:%M:%S',
                '%Y-%m-%d %H:%M',
                '%Y/%m/%d %H:%M:%S',
                '%Y/%m/%d %H:%M',
                '%m/%d/%Y %H:%M:%S',
                '%m/%d/%Y %H:%M',
                '%d/%m/%Y %H:%M:%S',
                '%d/%m/%Y %H:%M',
                '%Y-%m-%d',
                '%Y/%m/%d',
                '%m/%d/%Y',
                '%d/%m/%Y',
                '%d %b %Y',
                '%b %d, %Y',
                '%B %d, %Y'
            ]
            
            # Clean the deadline string
            deadline_str = deadline_str.replace('st', '').replace('nd', '').replace('rd', '').replace('th', '')
            
            for fmt in date_formats:
                try:
                    dt = datetime.strptime(deadline_str, fmt)
                    return int(dt.timestamp())
                except ValueError:
                    continue
            
            # Try to parse relative times like "5 days left"
            if 'days left' in deadline_str.lower():
                days_match = re.search(r'(\d+)\s+days?\s+left', deadline_str, re.IGNORECASE)
                if days_match:
                    days = int(days_match.group(1))
                    future_date = datetime.now() + timedelta(days=days)
                    return int(future_date.timestamp())
            
            if 'hours left' in deadline_str.lower():
                hours_match = re.search(r'(\d+)\s+hours?\s+left', deadline_str, re.IGNORECASE)
                if hours_match:
                    hours = int(hours_match.group(1))
                    future_date = datetime.now() + timedelta(hours=hours)
                    return int(future_date.timestamp())
            
            return None
        except Exception as e:
            logger.error(f"Error extracting deadline timestamp: {e}")
            return None

    def extract_participants(self, soup, driver):
        try:
            page_text = soup.get_text()

            participant_patterns = [
            r'(\d+(?:,\d+)*(?:\.\d+)?[KMB]?)\s*(?:participants?|users?|members?|joined|entries?)',
            r'(\d+(?:,\d+)*(?:\.\d+)?[KMB]?)\s*people',
            r'Participants?\s*:?\s*(\d+(?:,\d+)*(?:\.\d+)?[KMB]?)',
            r'Users?\s*:?\s*(\d+(?:,\d+)*(?:\.\d+)?[KMB]?)',
            r'Members?\s*:?\s*(\d+(?:,\d+)*(?:\.\d+)?[KMB]?)',
            r'(\d+(?:,\d+)*(?:\.\d+)?[KMB]?)\s*have\s+joined',
            r'(\d+(?:,\d+)*(?:\.\d+)?[KMB]?)\s*active\s+users?']

            for pattern in participant_patterns:
                matches = re.findall(pattern, page_text, re.IGNORECASE)
                if matches:
                    try:
                        count_str = matches[0]
                        if count_str.endswith('K'):
                            return int(float(count_str[:-1]) * 1000)
                        elif count_str.endswith('M'):
                            return int(float(count_str[:-1]) * 1000000)
                        elif count_str.endswith('B'):
                            return int(float(count_str[:-1]) * 1000000000)
                        else:
                            return int(count_str.replace(',', ''))
                    except (ValueError, AttributeError):
                        continue
            return 0
        except Exception as e:
            logger.error(f"Error extracting participants: {e}")
            return 0




    
    def extract_status(self, soup, driver):
        """Extract campaign status with improved detection"""
        try:
            page_text = soup.get_text().lower()
            
            # Status indicators
            if any(keyword in page_text for keyword in ['ended', 'expired', 'closed', 'finished']):
                return 'Ended'
            elif any(keyword in page_text for keyword in ['coming soon', 'upcoming', 'not started']):
                return 'Upcoming'
            elif any(keyword in page_text for keyword in ['live', 'active', 'ongoing', 'open']):
                return 'Live'
            else:
                # Check if there's a deadline in the future
                deadline_timestamp = self.extract_deadline_timestamp(soup, driver)
                if deadline_timestamp:
                    current_time = int(time.time())
                    if deadline_timestamp > current_time:
                        return 'Live'
                    else:
                        return 'Ended'
                return 'Unknown'
        except Exception as e:
            logger.error(f"Error extracting status: {e}")
            return 'Unknown'
    
    def extract_description(self, soup, driver):
        """Extract campaign description with improved detection"""
        try:
            # Try different selectors for description
            description_selectors = [
                'div[class*="description"]',
                'div[class*="about"]',
                'div[class*="details"]',
                'div[class*="content"]',
                'p[class*="description"]',
                'div[class*="summary"]',
                'div[class*="info"]'
            ]
            
            for selector in description_selectors:
                elem = soup.select_one(selector)
                if elem:
                    text = elem.get_text(strip=True)
                    if text and len(text) > 20:
                        # Clean up the text
                        text = re.sub(r'\s+', ' ', text)  # Replace multiple spaces with single space
                        return text[:500]  # Limit to 500 characters
            
            # Try to extract from meta description
            meta_desc = soup.find('meta', {'name': 'description'})
            if meta_desc and meta_desc.get('content'):
                return meta_desc['content'][:500]
            
            # Try to extract from first paragraph
            paragraphs = soup.find_all('p')
            for p in paragraphs:
                text = p.get_text(strip=True)
                if text and len(text) > 50:
                    return text[:500]
            
            return None
        except Exception as e:
            logger.error(f"Error extracting description: {e}")
            return None
    
    def extract_chain(self, soup, driver):
        """Extract blockchain chain with improved detection"""
        try:
            page_text = soup.get_text().lower()
            
            # Enhanced chain detection
            chain_indicators = {
                'Ethereum': ['ethereum', 'eth', 'mainnet', 'erc-20', 'erc20'],
                'BSC': ['bsc', 'binance smart chain', 'bnb chain', 'bep-20', 'bep20'],
                'Polygon': ['polygon', 'matic', 'poly'],
                'Arbitrum': ['arbitrum', 'arb'],
                'Optimism': ['optimism', 'op'],
                'Avalanche': ['avalanche', 'avax'],
                'Solana': ['solana', 'sol'],
                'Cardano': ['cardano', 'ada'],
                'Polkadot': ['polkadot', 'dot'],
                'Cosmos': ['cosmos', 'atom'],
                'Near': ['near protocol', 'near'],
                'Fantom': ['fantom', 'ftm'],
                'Harmony': ['harmony', 'one'],
                'Cronos': ['cronos', 'cro'],
                'Moonbeam': ['moonbeam', 'glmr'],
                'Kava': ['kava'],
                'Celo': ['celo'],
                'Gnosis': ['gnosis', 'xdai'],
                'Base': ['base chain', 'base']
            }
            
            detected_chains = []
            for chain, keywords in chain_indicators.items():
                for keyword in keywords:
                    if keyword in page_text:
                        detected_chains.append(chain)
                        break
            
            return ', '.join(detected_chains) if detected_chains else 'Unknown'
        except Exception as e:
            logger.error(f"Error extracting chain: {e}")
            return 'Unknown'
    
    def extract_estimated_value(self, soup, driver):
        """Extract estimated value with improved detection"""
        try:
            page_text = soup.get_text()
            
            # Enhanced value detection patterns
            value_patterns = [
                r'(\$\d+(?:,\d+)*(?:\.\d+)?(?:K|M|B)?)',
                r'(\d+(?:,\d+)*(?:\.\d+)?\s*(?:USD|USDT|USDC))',
                r'(\d+(?:,\d+)*(?:\.\d+)?\s*(?:ETH|BTC|BNB|SOL|MATIC|AVAX))',
                r'Value\s*:?\s*(\$?\d+(?:,\d+)*(?:\.\d+)?(?:K|M|B)?)',
                r'Worth\s*:?\s*(\$?\d+(?:,\d+)*(?:\.\d+)?(?:K|M|B)?)',
                r'Prize\s*:?\s*(\$?\d+(?:,\d+)*(?:\.\d+)?(?:K|M|B)?)',
                r'Total\s*:?\s*(\$?\d+(?:,\d+)*(?:\.\d+)?(?:K|M|B)?)'
            ]
            
            for pattern in value_patterns:
                matches = re.findall(pattern, page_text, re.IGNORECASE)
                if matches:
                    return matches[0]
            
            return None
        except Exception as e:
            logger.error(f"Error extracting estimated value: {e}")
            return None
    
    def extract_difficulty_level(self, soup, driver):
        """Extract difficulty level based on task complexity"""
        try:
            task_count = self.extract_task_count(soup, driver)
            task_types = self.extract_task_types(soup, driver)
            page_text = soup.get_text().lower()
            
            difficulty_score = 0
            
            # Score based on task count
            if task_count > 10:
                difficulty_score += 3
            elif task_count > 5:
                difficulty_score += 2
            elif task_count > 2:
                difficulty_score += 1
            
            # Score based on task complexity
            complex_tasks = ['on-chain', 'wallet connect', 'transaction', 'swap', 'stake', 'deploy']
            medium_tasks = ['quiz', 'referral', 'github']
            
            if any(task in task_types.lower() for task in complex_tasks):
                difficulty_score += 3
            elif any(task in task_types.lower() for task in medium_tasks):
                difficulty_score += 2
            
            # Score based on keywords in description
            if any(keyword in page_text for keyword in ['advanced', 'expert', 'complex', 'technical']):
                difficulty_score += 2
            elif any(keyword in page_text for keyword in ['beginner', 'easy', 'simple', 'basic']):
                difficulty_score -= 1
            
            # Determine difficulty level
            if difficulty_score >= 5:
                return 'Hard'
            elif difficulty_score >= 3:
                return 'Medium'
            else:
                return 'Easy'
        except Exception as e:
            logger.error(f"Error extracting difficulty level: {e}")
            return 'Unknown'
    
    def extract_is_featured(self, soup, driver):
        """Extract if campaign is featured"""
        try:
            page_text = soup.get_text().lower()
            
            # Look for featured indicators
            featured_indicators = [
                'featured',
                'spotlight',
                'highlighted',
                'promoted',
                'trending',
                'hot',
                'popular'
            ]
            
            return any(indicator in page_text for indicator in featured_indicators)
        except Exception as e:
            logger.error(f"Error extracting featured status: {e}")
            return False
    
    def save_campaign_to_db(self, campaign_data):
        """Save campaign data to database"""
        try:
            conn = sqlite3.connect(self.db_path)
            cursor = conn.cursor()
            
            cursor.execute('''
                INSERT OR REPLACE INTO campaigns (
                    campaign_id, project_name, campaign_title, campaign_url,
                    task_count, task_types, reward_type, reward_details,
                    deadline, deadline_timestamp, status, estimated_value,
                    description, chain, participants, is_featured,
                    difficulty_level, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ''', (
                campaign_data.get('campaign_id'),
                campaign_data.get('project_name'),
                campaign_data.get('campaign_title'),
                campaign_data.get('campaign_url'),
                campaign_data.get('task_count', 0),
                campaign_data.get('task_types'),
                campaign_data.get('reward_type'),
                campaign_data.get('reward_details'),
                campaign_data.get('deadline'),
                campaign_data.get('deadline_timestamp'),
                campaign_data.get('status'),
                campaign_data.get('estimated_value'),
                campaign_data.get('description'),
                campaign_data.get('chain'),
                campaign_data.get('participants', 0),
                campaign_data.get('is_featured', False),
                campaign_data.get('difficulty_level'),
            ))
            
            conn.commit()
            conn.close()
            logger.info(f"Campaign {campaign_data.get('campaign_id')} saved to database")
            return True
        except Exception as e:
            logger.error(f"Error saving campaign to database: {e}")
            return False
    
    def run_full_scrape(self, max_campaigns=50, max_scroll=15):
        """Run full scraping process"""
        logger.info("Starting full Galxe scraping process...")
        
        # Step 1: Get campaign URLs
        campaign_urls = self.scrape_campaign_urls(max_scroll=max_scroll)
        
        if not campaign_urls:
            logger.error("No campaign URLs found!")
            return
        
        logger.info(f"Found {len(campaign_urls)} campaigns to scrape")
        
        # Step 2: Scrape each campaign
        successful_scrapes = 0
        failed_scrapes = 0
        
        for i, url in enumerate(campaign_urls[:max_campaigns]):
            logger.info(f"Scraping campaign {i+1}/{min(len(campaign_urls), max_campaigns)}: {url}")
            
            try:
                campaign_data = self.extract_campaign_details(url)
                
                if campaign_data:
                    if self.save_campaign_to_db(campaign_data):
                        successful_scrapes += 1
                    else:
                        failed_scrapes += 1
                else:
                    failed_scrapes += 1
                
                # Add delay between requests
                time.sleep(random.uniform(2, 5))
                
            except Exception as e:
                logger.error(f"Error processing campaign {url}: {e}")
                failed_scrapes += 1
                continue
        
        logger.info(f"Scraping completed! Successful: {successful_scrapes}, Failed: {failed_scrapes}")
    
    def get_campaign_stats(self):
        """Get statistics from scraped campaigns"""
        try:
            conn = sqlite3.connect(self.db_path)
            cursor = conn.cursor()
            
            # Basic stats
            cursor.execute("SELECT COUNT(*) FROM campaigns")
            total_campaigns = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM campaigns WHERE status = 'Live'")
            live_campaigns = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM campaigns WHERE status = 'Ended'")
            ended_campaigns = cursor.fetchone()[0]
            
            cursor.execute("SELECT COUNT(*) FROM campaigns WHERE is_featured = 1")
            featured_campaigns = cursor.fetchone()[0]
            
            # Reward type distribution
            cursor.execute("SELECT reward_type, COUNT(*) FROM campaigns GROUP BY reward_type")
            reward_distribution = cursor.fetchall()
            
            # Chain distribution
            cursor.execute("SELECT chain, COUNT(*) FROM campaigns GROUP BY chain")
            chain_distribution = cursor.fetchall()
            
            # Task count distribution
            cursor.execute("SELECT difficulty_level, COUNT(*) FROM campaigns GROUP BY difficulty_level")
            difficulty_distribution = cursor.fetchall()
            
            conn.close()
            
            stats = {
                'total_campaigns': total_campaigns,
                'live_campaigns': live_campaigns,
                'ended_campaigns': ended_campaigns,
                'featured_campaigns': featured_campaigns,
                'reward_distribution': reward_distribution,
                'chain_distribution': chain_distribution,
                'difficulty_distribution': difficulty_distribution
            }
            
            return stats
        except Exception as e:
            logger.error(f"Error getting campaign stats: {e}")
            return None
    
    def export_campaigns_to_csv(self, filename='galxe_campaigns.csv'):
        """Export campaigns to CSV file"""
        try:
            import csv
            
            conn = sqlite3.connect(self.db_path)
            cursor = conn.cursor()
            
            cursor.execute('''
                SELECT * FROM campaigns 
                ORDER BY deadline_timestamp DESC, updated_at DESC
            ''')
            
            campaigns = cursor.fetchall()
            
            # Get column names
            cursor.execute("PRAGMA table_info(campaigns)")
            columns = [column[1] for column in cursor.fetchall()]
            
            conn.close()
            
            with open(filename, 'w', newline='', encoding='utf-8') as csvfile:
                writer = csv.writer(csvfile)
                writer.writerow(columns)
                writer.writerows(campaigns)
            
            logger.info(f"Exported {len(campaigns)} campaigns to {filename}")
            return True
        except Exception as e:
            logger.error(f"Error exporting campaigns to CSV: {e}")
            return False
    
    def cleanup_old_campaigns(self, days_old=30):
        """Clean up old campaigns from database"""
        try:
            conn = sqlite3.connect(self.db_path)
            cursor = conn.cursor()
            
            cutoff_timestamp = int((datetime.now() - timedelta(days=days_old)).timestamp())
            
            cursor.execute('''
                DELETE FROM campaigns 
                WHERE deadline_timestamp < ? AND status = 'Ended'
            ''', (cutoff_timestamp,))
            
            deleted_count = cursor.rowcount
            conn.commit()
            conn.close()
            
            logger.info(f"Cleaned up {deleted_count} old campaigns")
            return deleted_count
        except Exception as e:
            logger.error(f"Error cleaning up old campaigns: {e}")
            return 0

# Usage example
if __name__ == "__main__":
    scraper = EnhancedGalxeScraper()
    
    # Run full scraping process
    scraper.run_full_scrape(max_campaigns=100, max_scroll=20)
    
    # Get and print statistics
    stats = scraper.get_campaign_stats()
    if stats:
        print("\n=== GALXE CAMPAIGN STATISTICS ===")
        print(f"Total Campaigns: {stats['total_campaigns']}")
        print(f"Live Campaigns: {stats['live_campaigns']}")
        print(f"Ended Campaigns: {stats['ended_campaigns']}")
        print(f"Featured Campaigns: {stats['featured_campaigns']}")
        
        print("\nReward Distribution:")
        for reward_type, count in stats['reward_distribution']:
            print(f"  {reward_type}: {count}")
        
        print("\nChain Distribution:")
        for chain, count in stats['chain_distribution']:
            print(f"  {chain}: {count}")
        
        print("\nDifficulty Distribution:")
        for difficulty, count in stats['difficulty_distribution']:
            print(f"  {difficulty}: {count}")
    
    # Export to CSV
    scraper.export_campaigns_to_csv()
    
    # Clean up old campaigns
    scraper.cleanup_old_campaigns(days_old=30)
