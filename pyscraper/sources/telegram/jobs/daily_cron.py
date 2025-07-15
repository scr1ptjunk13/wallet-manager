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
    # Ensure the data directory exists
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

    # Step 2: Combine message texts and parse
    combined_text = "\n\n".join([msg["text"] for msg in messages])
    campaigns = parse_telegram_message(combined_text, channel="airdrops_io", first_seen="cron" if last_timestamp else "initial")
    
    print("Parsed campaigns:", campaigns)

    new_latest_ts = last_timestamp

    # Step 3: Process each campaign
    for campaign in campaigns:
        try:
            if not campaign:
                print("Empty campaign, skipping")
                continue

            print(f"Processing: {campaign['airdrop_name']}")
            airdrop_data = extract_info_from_airdrop_page(campaign["link"])
            if not airdrop_data:
                print("No data found on page, skipping")
                continue

            enriched = {
                **campaign,
                **airdrop_data,
                "fetched_at": datetime.utcnow().isoformat()
            }
            insert_campaign(enriched)

            if campaign["telegram_timestamp"] > new_latest_ts:
                new_latest_ts = campaign["telegram_timestamp"]

        except Exception as e:
            print("Failed to process campaign:", e)
            continue

    # Step 4: Update pointer
    pointer["airdrops_io"] = new_latest_ts
    save_pointer(pointer)
    print("Pointer updated")

if __name__ == "__main__":
    main()

