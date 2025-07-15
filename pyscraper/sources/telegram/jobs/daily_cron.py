import os
import sys
import json
from datetime import datetime

# Add the pyscraper directory to Python path
current_dir = os.path.dirname(os.path.abspath(__file__))
pyscraper_dir = os.path.join(current_dir, '..', '..', '..')
sys.path.insert(0, os.path.abspath(pyscraper_dir))

# Correct imports based on your actual directory structure
from sources.telegram.telegram.fetch_messages import fetch_new_messages
from sources.telegram.telegram.parse_message import parse_telegram_message
from sources.telegram.webcrawler.crawl_site import extract_info_from_airdrop_page
from sources.telegram.data.init_db import insert_campaign, init_db

POINTER_PATH = "sources/telegram/data/pointer.json"

def load_pointer():
    if os.path.exists(POINTER_PATH) and os.path.getsize(POINTER_PATH) > 0:
        with open(POINTER_PATH, "r") as f:
            return json.load(f)
    return {}

def save_pointer(pointer):
    os.makedirs(os.path.dirname(POINTER_PATH), exist_ok=True)
    with open(POINTER_PATH, "w") as f:
        json.dump(pointer, f, indent=2)

def main():
    init_db()
    pointer = load_pointer()
    last_timestamp = pointer.get("airdrops_io", "1970-01-01T00:00:00")

    # Step 1: Fetch new messages since last pointer
    messages = fetch_new_messages(since=last_timestamp)
    print(f"Fetched {len(messages)} new messages")

    all_campaigns = []
    new_latest_ts = last_timestamp

    # Step 2: Parse each message individually
    for msg in messages:
        try:
            campaigns = parse_telegram_message(
                msg["text"], 
                channel="airdrops_io", 
                first_seen="cron"
            )

            for campaign in campaigns:
                campaign["telegram_timestamp"] = msg["timestamp"]
                all_campaigns.append(campaign)

                # Update pointer time if needed
                if msg["timestamp"] > new_latest_ts:
                    new_latest_ts = msg["timestamp"]

        except Exception as e:
            print("Failed to parse message:", e)
            continue

    print(f"Parsed {len(all_campaigns)} campaigns")

    # Step 3: Enrich and insert campaigns
    for campaign in all_campaigns:
        try:
            print(f"Processing: {campaign['airdrop_name']}")
            airdrop_data = extract_info_from_airdrop_page(campaign["link"])
            if not airdrop_data:
                continue

            enriched = {
                **campaign,
                **airdrop_data,
                "fetched_at": datetime.utcnow().isoformat()
            }
            insert_campaign(enriched)

        except Exception as e:
            print("Failed to enrich/insert campaign:", e)
            continue

    # Step 4: Update pointer only if new data was processed
    if all_campaigns:
        pointer["airdrops_io"] = new_latest_ts
        save_pointer(pointer)
        print("Pointer updated")
    else:
        print("No campaigns to update. Pointer not changed.")

if __name__ == "__main__":
    main()

