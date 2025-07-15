# File: pyscraper/telegram/parse_message.py

import re
from datetime import datetime
from typing import List, Dict, Optional


def parse_telegram_message(raw: str, channel: str, first_seen: Optional[str] = None) -> List[Dict]:
    """
    Parses raw Telegram message block and extracts:
    - Airdrop name
    - Airdrop link
    - Timestamp of original message
    - Whether it was first seen (initial scan or cron job)
    - Channel name
    
    Returns: List[Dict]
    """
    blocks = re.split(r"AIRDROPS.IO \ud83d\ude80, \[.*?\]", raw)
    timestamps = re.findall(r"\[.*?\]", raw)

    campaigns = []
    for i, block in enumerate(blocks[1:]):  # First split part is always empty
        timestamp_str = timestamps[i].strip("[]")
        dt = datetime.strptime(timestamp_str, "%m/%d/%y %I:%M\u202f%p")
        links = re.findall(r"https?://\S+", block)

        for link in links:
            # Extract name heuristically
            name_match = re.search(r"(?:https?://)?(?:www\.)?([\w\-\.]+)\.\w+", link)
            name = name_match.group(1).capitalize() if name_match else "Unknown"

            campaigns.append({
                "airdrop_name": name,
                "link": link,
                "telegram_timestamp": dt.isoformat(),
                "scan_type": "initial" if first_seen == "initial" else "cron",
                "channel": channel,
                "raw_text": block.strip()
            })

    return campaigns

