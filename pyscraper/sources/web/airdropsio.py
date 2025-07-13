import requests
from bs4 import BeautifulSoup
import json
import time
import sqlite3
from datetime import datetime
import hashlib
import re
from urllib.parse import urljoin, urlparse
import logging

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class AirdropsIOScraper:
    def __init__(self, db_path='airdrops.db'):
        self.base_url = 'https://airdrops.io'
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
            'Accept-Language': 'en-US,en;q=0.5',
            'Accept-Encoding': 'gzip, deflate',
            'Connection': 'keep-alive',
            'Upgrade-Insecure-Requests': '1'
        })
        self.db_path = db_path
        self.init_database()

    def init_database(self):
        """Initialize SQLite database with airdrops table"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute('''
                       CREATE TABLE IF NOT EXISTS airdrops (
                                                               id INTEGER PRIMARY KEY AUTOINCREMENT,
                                                               project_name TEXT NOT NULL,
                                                               tags TEXT,
                                                               requirements TEXT,
                                                               reward_type TEXT,
                                                               reward_value TEXT,
                                                               deadline TEXT,
                                                               social_links TEXT,
                                                               source_link TEXT,
                                                               content_hash TEXT UNIQUE,
                                                               scraped_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                                                               section TEXT
                       )
                       ''')

        conn.commit()
        conn.close()

    def get_content_hash(self, content):
        """Generate hash for deduplication"""
        return hashlib.md5(content.encode()).hexdigest()

    def save_to_db(self, airdrop_data):
        """Save airdrop data to database with deduplication"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        try:
            cursor.execute('''
                           INSERT OR IGNORE INTO airdrops 
                (project_name, tags, requirements, reward_type, reward_value, 
                 deadline, social_links, source_link, content_hash, section)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                           ''', (
                               airdrop_data['project_name'],
                               json.dumps(airdrop_data['tags']),
                               json.dumps(airdrop_data['requirements']),
                               airdrop_data['reward_type'],
                               airdrop_data['reward_value'],
                               airdrop_data['deadline'],
                               json.dumps(airdrop_data['social_links']),
                               airdrop_data['source_link'],
                               airdrop_data['content_hash'],
                               airdrop_data['section']
                           ))

            if cursor.rowcount > 0:
                logger.info(f"Saved new airdrop: {airdrop_data['project_name']}")
            else:
                logger.debug(f"Duplicate airdrop skipped: {airdrop_data['project_name']}")

        except Exception as e:
            logger.error(f"Error saving to database: {e}")

        conn.commit()
        conn.close()

    def extract_social_links(self, soup):
        """Extract social media links from the airdrop page"""
        social_links = {}

        # Common social media patterns
        social_patterns = {
            'twitter': r'twitter\.com/\w+',
            'discord': r'discord\.gg/\w+|discord\.com/invite/\w+',
            'telegram': r't\.me/\w+',
            'website': r'https?://[\w\.-]+\.[a-zA-Z]{2,}'
        }

        # Look for links in the page
        links = soup.find_all('a', href=True)
        for link in links:
            href = link.get('href', '')
            for platform, pattern in social_patterns.items():
                if re.search(pattern, href, re.IGNORECASE):
                    social_links[platform] = href
                    break

        return social_links

    def extract_tags(self, soup):
        """Extract tags/categories from the airdrop"""
        tags = []

        # Look for common tag indicators
        tag_selectors = [
            '.tag', '.category', '.label', '.badge',
            '[class*="tag"]', '[class*="category"]'
        ]

        for selector in tag_selectors:
            elements = soup.select(selector)
            for element in elements:
                tag_text = element.get_text(strip=True)
                if tag_text and len(tag_text) < 50:  # Filter out long text
                    tags.append(tag_text)

        return list(set(tags))  # Remove duplicates

    def extract_requirements(self, soup):
        """Extract requirements from the airdrop description"""
        requirements = []

        # Look for common requirement keywords
        requirement_keywords = [
            'twitter', 'discord', 'telegram', 'follow', 'join',
            'testnet', 'mainnet', 'mint', 'nft', 'stake',
            'connect wallet', 'bridge', 'swap', 'trade'
        ]

        text_content = soup.get_text().lower()
        for keyword in requirement_keywords:
            if keyword in text_content:
                requirements.append(keyword)

        # Look for specific requirement patterns
        requirement_patterns = [
            r'follow @\w+',
            r'join.*discord',
            r'mint.*nft',
            r'bridge.*tokens?',
            r'complete.*tasks?'
        ]

        for pattern in requirement_patterns:
            matches = re.findall(pattern, text_content, re.IGNORECASE)
            requirements.extend(matches)

        return list(set(requirements))

    def extract_deadline(self, soup):
        """Extract deadline information"""
        deadline_patterns = [
            r'deadline:?\s*(\d{1,2}[\/\-]\d{1,2}[\/\-]\d{2,4})',
            r'ends?:?\s*(\d{1,2}[\/\-]\d{1,2}[\/\-]\d{2,4})',
            r'until:?\s*(\d{1,2}[\/\-]\d{1,2}[\/\-]\d{2,4})'
        ]

        text_content = soup.get_text()
        for pattern in deadline_patterns:
            match = re.search(pattern, text_content, re.IGNORECASE)
            if match:
                return match.group(1)

        return None

    def extract_reward_info(self, soup):
        """Extract reward type and value"""
        reward_type = "Unknown"
        reward_value = None

        # Look for token names and values
        token_patterns = [
            r'(\d+(?:,\d+)*(?:\.\d+)?)\s*([A-Z]{2,10})\s*tokens?',
            r'([A-Z]{2,10})\s*tokens?.*?(\d+(?:,\d+)*(?:\.\d+)?)',
            r'\$(\d+(?:,\d+)*(?:\.\d+)?)\s*(?:worth|value|USDT|USD)'
        ]

        text_content = soup.get_text()
        for pattern in token_patterns:
            match = re.search(pattern, text_content, re.IGNORECASE)
            if match:
                if '$' in pattern:
                    reward_value = f"${match.group(1)}"
                    reward_type = "USD Value"
                else:
                    reward_value = match.group(1)
                    reward_type = match.group(2) if len(match.groups()) > 1 else "Token"
                break

        return reward_type, reward_value

    def scrape_airdrop_details(self, airdrop_url):
        """Scrape detailed information from individual airdrop page"""
        try:
            response = self.session.get(airdrop_url, timeout=10)
            response.raise_for_status()

            soup = BeautifulSoup(response.content, 'html.parser')

            # Extract project name from title or heading
            project_name = "Unknown"
            title_selectors = ['h1', '.title', '.project-name', 'title']
            for selector in title_selectors:
                element = soup.select_one(selector)
                if element:
                    project_name = element.get_text(strip=True)
                    break

            # Extract all information
            tags = self.extract_tags(soup)
            requirements = self.extract_requirements(soup)
            reward_type, reward_value = self.extract_reward_info(soup)
            deadline = self.extract_deadline(soup)
            social_links = self.extract_social_links(soup)

            # Create content hash for deduplication
            content_for_hash = f"{project_name}{tags}{requirements}{reward_type}"
            content_hash = self.get_content_hash(content_for_hash)

            return {
                'project_name': project_name,
                'tags': tags,
                'requirements': requirements,
                'reward_type': reward_type,
                'reward_value': reward_value,
                'deadline': deadline,
                'social_links': social_links,
                'source_link': airdrop_url,
                'content_hash': content_hash
            }

        except Exception as e:
            logger.error(f"Error scraping airdrop details from {airdrop_url}: {e}")
            return None

    def scrape_section(self, section_url, section_name):
        """Scrape a specific section (latest, hot, etc.)"""
        try:
            logger.info(f"Scraping {section_name} section: {section_url}")
            response = self.session.get(section_url, timeout=10)
            response.raise_for_status()

            soup = BeautifulSoup(response.content, 'html.parser')

            # Find airdrop links - adapt these selectors based on actual HTML structure
            airdrop_links = []

            # Common selectors for airdrop cards/items
            link_selectors = [
                'a[href*="/airdrop/"]',
                'a[href*="/airdrops/"]',
                '.airdrop-card a',
                '.airdrop-item a',
                'article a'
            ]

            for selector in link_selectors:
                elements = soup.select(selector)
                for element in elements:
                    href = element.get('href')
                    if href:
                        full_url = urljoin(self.base_url, href)
                        airdrop_links.append(full_url)

            # Remove duplicates
            airdrop_links = list(set(airdrop_links))

            logger.info(f"Found {len(airdrop_links)} airdrops in {section_name}")

            # Scrape each airdrop
            scraped_count = 0
            for link in airdrop_links:
                airdrop_data = self.scrape_airdrop_details(link)
                if airdrop_data:
                    airdrop_data['section'] = section_name
                    self.save_to_db(airdrop_data)
                    scraped_count += 1

                # Be respectful - add delay between requests
                time.sleep(1)

            logger.info(f"Successfully scraped {scraped_count} airdrops from {section_name}")
            return scraped_count

        except Exception as e:
            logger.error(f"Error scraping {section_name} section: {e}")
            return 0

    def scrape_all_sections(self):
        """Scrape all main sections"""
        sections = {
            'latest': '/latest',
            'hot': '/hot',
            'potential': '/potential'
        }

        total_scraped = 0
        for section_name, section_path in sections.items():
            section_url = urljoin(self.base_url, section_path)
            count = self.scrape_section(section_url, section_name)
            total_scraped += count

            # Delay between sections
            time.sleep(2)

        logger.info(f"Total airdrops scraped: {total_scraped}")
        return total_scraped

    def get_stored_airdrops(self, limit=None):
        """Retrieve stored airdrops from database"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        query = '''
                SELECT project_name, tags, requirements, reward_type, reward_value,
                       deadline, social_links, source_link, scraped_at, section
                FROM airdrops
                ORDER BY scraped_at DESC \
                '''

        if limit:
            query += f' LIMIT {limit}'

        cursor.execute(query)
        rows = cursor.fetchall()

        airdrops = []
        for row in rows:
            airdrop = {
                'project_name': row[0],
                'tags': json.loads(row[1]) if row[1] else [],
                'requirements': json.loads(row[2]) if row[2] else [],
                'reward_type': row[3],
                'reward_value': row[4],
                'deadline': row[5],
                'social_links': json.loads(row[6]) if row[6] else {},
                'source_link': row[7],
                'scraped_at': row[8],
                'section': row[9]
            }
            airdrops.append(airdrop)

        conn.close()
        return airdrops

