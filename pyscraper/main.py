from scrapers.airdrops_io import AirdropsIOScraper

def main():
    """Main function to run the scraper"""
    scraper = AirdropsIOScraper()
    
    # Scrape all sections
    scraper.scrape_all_sections()
    
    # Display some results
    airdrops = scraper.get_stored_airdrops(limit=10)
    
    print(f"\n=== Latest {len(airdrops)} Airdrops ===")
    for airdrop in airdrops:
        print(f"\n📊 {airdrop['project_name']}")
        print(f"🏷️  Tags: {', '.join(airdrop['tags'])}")
        print(f"📋 Requirements: {', '.join(airdrop['requirements'])}")
        print(f"🎁 Reward: {airdrop['reward_type']} {airdrop['reward_value'] or ''}")
        print(f"⏰ Deadline: {airdrop['deadline'] or 'Not specified'}")
        print(f"🔗 Source: {airdrop['source_link']}")
        print(f"📅 Scraped: {airdrop['scraped_at']}")
        print("-" * 50)

if __name__ == "__main__":
    main()
