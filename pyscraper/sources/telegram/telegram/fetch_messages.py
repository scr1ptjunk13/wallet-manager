from dotenv import load_dotenv
load_dotenv()

from telethon.sync import TelegramClient
from datetime import datetime
import os

API_ID = os.getenv("TG_API_ID")
API_HASH = os.getenv("TG_API_HASH")
SESSION_NAME = "airdrop_scraper"
CHANNEL_USERNAME = "airdrops_io"

def fetch_new_messages(since=None):
    """
    Fetch new Telegram messages from CHANNEL_USERNAME after 'since' timestamp.
    Returns a list of dicts with 'text' and 'timestamp'.
    """
    with TelegramClient(SESSION_NAME, API_ID, API_HASH) as client:
        entity = client.get_entity(CHANNEL_USERNAME)

        all_msgs = []
        for message in client.iter_messages(entity, limit=100):
            if since and message.date.isoformat() <= since:
                break
            if message.raw_text:
                all_msgs.append({
                    "text": message.raw_text,
                    "timestamp": message.date.isoformat()
                })

        return list(reversed(all_msgs))  # Return oldest to newest

if __name__ == "__main__":
    msgs = fetch_new_messages()
    for m in msgs:
        print(f"[{m['timestamp']}] {m['text'][:80]}")

