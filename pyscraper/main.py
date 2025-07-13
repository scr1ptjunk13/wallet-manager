from scrapers.airdrops_io import AirdropsIOScraper
from scrapers.galaxe import EnhancedGalxeScraper
from utils.campaign_database import CampaignDatabase

def print_airdrop_summary(airdrops, limit=10):
    print(f"\n=== Latest {len(airdrops)} Airdrops from Airdrops.io ===")
    for airdrop in airdrops[:limit]:
        print(f"\n📊 {airdrop['project_name']}")
        print(f"🏷️  Tags: {', '.join(airdrop['tags'])}")
        print(f"📋 Requirements: {', '.join(airdrop['requirements'])}")
        print(f"🎁 Reward: {airdrop['reward_type']} {airdrop['reward_value'] or ''}")
        print(f"⏰ Deadline: {airdrop['deadline'] or 'Not specified'}")
        print(f"🔗 Source: {airdrop['source_link']}")
        print(f"📅 Scraped: {airdrop['scraped_at']}")
        print("-" * 50)

def print_galxe_summary(campaigns, limit=10, earndrop_only=False):
    campaign_type = "EARNDROP" if earndrop_only else "ALL"
    print(f"\n=== GALXE {campaign_type} CAMPAIGNS SUMMARY (Top {limit}) ===")
    print("-" * 80)
    for i, campaign in enumerate(campaigns[:limit], 1):
        earndrop_badge = " [EARNDROP]" if campaign.get('is_earndrop') else ""
        print(f"{i}. {campaign['campaign_title'] or 'Untitled'}{earndrop_badge}")
        print(f"   Project: {campaign['project_name'] or 'Unknown'}")
        print(f"   Tasks: {campaign['task_count'] or 0} ({campaign['task_types'] or 'Unknown types'})")
        print(f"   Reward: {campaign['reward_type'] or 'Unknown'}")
        print(f"   Participants: {campaign['participants'] or 'Unknown'}")
        print(f"   Chain: {campaign['chain'] or 'Unknown'}")
        print(f"   Status: {campaign['status'] or 'Unknown'}")
        print(f"   URL: {campaign['campaign_url'] or 'N/A'}")
        print(f"   Scraped: {campaign['scraped_at']}")
        print("-" * 80)

def main():
    print("\n=== Airdrop Scraper ===")
    print("Select a platform to scrape:")
    print("[1] Airdrops.io")
    print("[2] Galxe")

    while True:
        choice = input("\nEnter your choice (1 or 2): ").strip()

        if choice == '1':
            print("\nScraping Airdrops.io...")
            scraper = AirdropsIOScraper()
            scraper.scrape_all_sections()
            airdrops = scraper.get_stored_airdrops(limit=10)
            if not airdrops:
                print("\nNo airdrops found. Check logs for errors or try again later.")
            else:
                print_airdrop_summary(airdrops)
            break

        elif choice == '2':
            print("\nScraping Galxe...")
            scraper = EnhancedGalxeScraper()
            scraper.run_full_scrape(max_campaigns=100, max_scroll=20)

            db = CampaignDatabase()
            campaigns = db.get_campaigns(limit=100)

            if not campaigns:
                print("\nNo campaigns found. Possible issues:")
                print("- Check if the Galxe API endpoint (https://graphigo.prd.galaxy.eco/query) is correct.")
                print("- Verify the earndrop URL (https://app.galxe.com/quest).")
                print("- Inspect website HTML for updated campaign selectors.")
                print("Check logs for detailed errors.")
                break

            print("\nREGULAR CAMPAIGNS")
            regular_campaigns = db.get_campaigns(limit=5, status='active')
            if regular_campaigns:
                print_galxe_summary(regular_campaigns, limit=5, earndrop_only=False)
            else:
                print("No regular campaigns found in database.")

            print("\nEARNDROP CAMPAIGNS (HIGH ALPHA)")
            earndrop_campaigns = [c for c in regular_campaigns if c.get('is_earndrop')]
            if earndrop_campaigns:
                print_galxe_summary(earndrop_campaigns, limit=5, earndrop_only=True)
            else:
                print("No earndrop campaigns found in database.")

            print("\nHIGH VALUE CAMPAIGNS (>1000 participants)")
            high_value = [c for c in regular_campaigns if c.get('participants') and c['participants'] > 1000]
            if high_value:
                for i, campaign in enumerate(high_value[:10], 1):
                    print(f"{i}. {campaign['campaign_title']} - {campaign['participants']} participants")
            else:
                print("No high-value campaigns found in database.")
            break

        else:
            print("Invalid choice! Please enter 1 or 2.")

if __name__ == "__main__":
    main()

