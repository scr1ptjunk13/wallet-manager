import asyncio
import json
import re
from datetime import datetime
from typing import List, Dict, Optional
from twscrape import API, gather
from twscrape.logger import set_log_level
import logging

class AirdropScraper:
    def __init__(self, db_path: str = "airdrop_accounts.db"):
        self.api = API(db_path)
        self.airdrop_keywords = [
            "airdrop", "testnet", "mainnet", "farming", "alpha", "whitelist",
            "retroactive", "snapshot", "claim", "eligibility", "rewards",
            "points", "tier", "multiplier", "referral", "invite", "early access",
            "beta", "launch", "TGE", "token generation", "allocation"
        ]
        
        # Common airdrop-related accounts to monitor
        self.airdrop_accounts = [
            "AirdropDetective", "DeFiKingdoms", "LayerZero_Labs", 
            "StarkWareLtd", "zksync", "Scroll_ZKP", "LineaBuild",
            "arbitrum", "optimismFND", "ethereum", "Polygon",
            "avalancheavax", "0xPolygon", "SuiNetwork", "AptosLabs"
        ]
        
        # Setup logging
        logging.basicConfig(
            level=logging.INFO,
            format='%(asctime)s - %(levelname)s - %(message)s',
            handlers=[
                logging.FileHandler('airdrop_scraper.log'),
                logging.StreamHandler()
            ]
        )
        self.logger = logging.getLogger(__name__)

    async def setup_accounts(self):
        """Setup Twitter accounts from accounts.txt file"""
        print("Setting up accounts from accounts.txt...")
        
        try:
            # Check if accounts.txt exists
            import os
            if not os.path.exists('accounts.txt'):
                print("❌ accounts.txt file not found!")
                print("Please create accounts.txt with format:")
                print("username:password:email:email_password:cookies")
                return False
            
            # Read accounts from file
            with open('accounts.txt', 'r') as f:
                lines = f.read().strip().split('\n')
            
            accounts_added = 0
            for line in lines:
                if line.strip() and not line.startswith('#'):
                    parts = line.split(':', 4)  # Split into max 5 parts
                    if len(parts) >= 4:
                        username = parts[0]
                        password = parts[1]
                        email = parts[2]
                        email_password = parts[3]
                        cookies = parts[4] if len(parts) > 4 else None
                        
                        if cookies:
                            await self.api.pool.add_account(username, password, email, email_password, cookies=cookies)
                        else:
                            await self.api.pool.add_account(username, password, email, email_password)
                        
                        accounts_added += 1
                        print(f"✅ Added account: {username}")
            
            print(f"Successfully added {accounts_added} accounts")
            
            # Try to login accounts without cookies
            try:
                await self.api.pool.login_all()
                print("✅ Login completed")
            except Exception as e:
                print(f"⚠️ Login warning: {e}")
                print("If you're using cookies, this is normal")
            
            return True
            
        except Exception as e:
            print(f"❌ Error setting up accounts: {e}")
            return False

    def is_airdrop_related(self, tweet_content: str) -> bool:
        """Check if tweet is airdrop-related"""
        content_lower = tweet_content.lower()
        return any(keyword in content_lower for keyword in self.airdrop_keywords)

    def extract_airdrop_info(self, tweet) -> Dict:
        """Extract structured airdrop information from tweet"""
        content = tweet.rawContent
        
        # Extract potential project names (usually capitalized or with $)
        project_names = re.findall(r'[A-Z][a-zA-Z]+|[\$][A-Z]+', content)
        
        # Extract URLs
        urls = re.findall(r'https?://[^\s]+', content)
        
        # Extract dates
        dates = re.findall(r'\d{1,2}[/-]\d{1,2}[/-]\d{2,4}|\d{1,2}\s+\w+\s+\d{4}', content)
        
        # Check for urgency indicators
        urgency_keywords = ['soon', 'ending', 'deadline', 'last chance', 'limited time']
        is_urgent = any(keyword in content.lower() for keyword in urgency_keywords)
        
        # Extract potential reward amounts
        rewards = re.findall(r'[\$€£¥]\d+(?:,\d{3})*(?:\.\d{2})?|[\d,]+\s*(?:tokens?|coins?|USD|ETH|BTC)', content)
        
        return {
            'tweet_id': tweet.id,
            'user': tweet.user.username,
            'display_name': tweet.user.displayname,
            'content': content,
            'created_at': tweet.date,
            'likes': tweet.likeCount,
            'retweets': tweet.retweetCount,
            'replies': tweet.replyCount,
            'views': tweet.viewCount,
            'url': f"https://twitter.com/{tweet.user.username}/status/{tweet.id}",
            'project_names': project_names,
            'urls': urls,
            'dates': dates,
            'is_urgent': is_urgent,
            'potential_rewards': rewards,
            'verified_user': tweet.user.verified
        }

    async def search_airdrop_tweets(self, query: str, limit: int = 50) -> List[Dict]:
        """Search for airdrop-related tweets"""
        self.logger.info(f"Searching for: {query}")
        
        try:
            tweets = await gather(self.api.search(query, limit=limit))
            airdrop_tweets = []
            
            for tweet in tweets:
                if self.is_airdrop_related(tweet.rawContent):
                    airdrop_info = self.extract_airdrop_info(tweet)
                    airdrop_tweets.append(airdrop_info)
            
            self.logger.info(f"Found {len(airdrop_tweets)} airdrop-related tweets")
            return airdrop_tweets
            
        except Exception as e:
            self.logger.error(f"Error searching tweets: {e}")
            return []

    async def monitor_airdrop_accounts(self, limit: int = 20) -> List[Dict]:
        """Monitor specific airdrop-related accounts"""
        all_tweets = []
        
        for username in self.airdrop_accounts:
            self.logger.info(f"Monitoring @{username}")
            
            try:
                user = await self.api.user_by_login(username)
                tweets = await gather(self.api.user_tweets(user.id, limit=limit))
                
                for tweet in tweets:
                    if self.is_airdrop_related(tweet.rawContent):
                        airdrop_info = self.extract_airdrop_info(tweet)
                        all_tweets.append(airdrop_info)
                        
            except Exception as e:
                self.logger.error(f"Error monitoring @{username}: {e}")
                continue
        
        return all_tweets

    async def get_trending_airdrops(self) -> List[Dict]:
        """Get trending airdrop topics"""
        try:
            trends = await gather(self.api.trends("crypto"))
            airdrop_trends = []
            
            for trend in trends:
                if any(keyword in trend.name.lower() for keyword in self.airdrop_keywords):
                    airdrop_trends.append({
                        'name': trend.name,
                        'url': trend.url,
                        'tweet_volume': getattr(trend, 'tweet_volume', 'N/A')
                    })
            
            return airdrop_trends
            
        except Exception as e:
            self.logger.error(f"Error getting trends: {e}")
            return []

    async def comprehensive_airdrop_search(self, hours_back: int = 24) -> Dict:
        """Perform comprehensive airdrop search"""
        
        # Define search queries
        search_queries = [
            "airdrop alpha",
            "testnet airdrop",
            "mainnet airdrop",
            "airdrop farming",
            "retroactive airdrop",
            "whitelist airdrop",
            "new airdrop",
            "airdrop announcement",
            "claim airdrop",
            "airdrop snapshot"
        ]
        
        all_results = {
            'search_results': [],
            'account_monitoring': [],
            'trending_airdrops': [],
            'summary': {
                'total_tweets': 0,
                'urgent_opportunities': 0,
                'verified_sources': 0,
                'timestamp': datetime.now().isoformat()
            }
        }
        
        # Search for each query
        for query in search_queries:
            results = await self.search_airdrop_tweets(query, limit=30)
            all_results['search_results'].extend(results)
        
        # Monitor specific accounts
        account_results = await self.monitor_airdrop_accounts(limit=10)
        all_results['account_monitoring'] = account_results
        
        # Get trending airdrops
        trending = await self.get_trending_airdrops()
        all_results['trending_airdrops'] = trending
        
        # Calculate summary
        all_tweets = all_results['search_results'] + all_results['account_monitoring']
        all_results['summary']['total_tweets'] = len(all_tweets)
        all_results['summary']['urgent_opportunities'] = sum(1 for tweet in all_tweets if tweet['is_urgent'])
        all_results['summary']['verified_sources'] = sum(1 for tweet in all_tweets if tweet['verified_user'])
        
        return all_results

    def save_results(self, results: Dict, filename: str = None):
        """Save results to JSON file"""
        if filename is None:
            filename = f"airdrop_results_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        
        with open(filename, 'w', encoding='utf-8') as f:
            json.dump(results, f, indent=2, ensure_ascii=False, default=str)
        
        self.logger.info(f"Results saved to {filename}")

    def filter_high_value_opportunities(self, results: Dict) -> List[Dict]:
        """Filter high-value airdrop opportunities"""
        all_tweets = results['search_results'] + results['account_monitoring']
        
        high_value = []
        for tweet in all_tweets:
            # High-value criteria
            if (tweet['verified_user'] or 
                tweet['likes'] > 100 or 
                tweet['retweets'] > 50 or
                tweet['is_urgent'] or
                any(url for url in tweet['urls'] if 'testnet' in url.lower() or 'mainnet' in url.lower())):
                high_value.append(tweet)
        
        # Sort by engagement
        high_value.sort(key=lambda x: x['likes'] + x['retweets'], reverse=True)
        return high_value

async def main():
    """Main function to run the airdrop scraper"""
    scraper = AirdropScraper()
    
    print("🪂 Airdrop Alpha Scraper for X/Twitter 🪂")
    print("=" * 50)
    
    # Setup accounts from accounts.txt
    if not await scraper.setup_accounts():
        print("❌ Failed to setup accounts. Please check your accounts.txt file.")
        return
    
    try:
        print("\n🔍 Starting comprehensive airdrop search...")
        
        # Perform comprehensive search
        results = await scraper.comprehensive_airdrop_search()
        
        # Filter high-value opportunities
        high_value = scraper.filter_high_value_opportunities(results)
        
        # Display summary
        print(f"\n📊 SUMMARY")
        print(f"Total tweets found: {results['summary']['total_tweets']}")
        print(f"Urgent opportunities: {results['summary']['urgent_opportunities']}")
        print(f"Verified sources: {results['summary']['verified_sources']}")
        print(f"High-value opportunities: {len(high_value)}")
        
        # Display top opportunities
        print(f"\n🎯 TOP AIRDROP OPPORTUNITIES:")
        for i, tweet in enumerate(high_value[:5], 1):
            print(f"\n{i}. @{tweet['user']} ({tweet['likes']} likes, {tweet['retweets']} RTs)")
            print(f"   {tweet['content'][:100]}...")
            print(f"   URL: {tweet['url']}")
            if tweet['urls']:
                print(f"   Links: {', '.join(tweet['urls'][:2])}")
        
        # Save results
        scraper.save_results(results)
        
        print(f"\n💾 Results saved to file!")
        print(f"🎉 Scraping completed successfully!")
        
    except Exception as e:
        print(f"❌ Error during scraping: {e}")
        scraper.logger.error(f"Scraping error: {e}")
        print("Please check your account credentials and try again.")

if __name__ == "__main__":
    # Set log level
    set_log_level("INFO")
    
    # Run the scraper
    asyncio.run(main())
