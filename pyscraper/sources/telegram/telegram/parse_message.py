import re
from datetime import datetime
from typing import List, Dict, Optional

def parse_telegram_message(raw: str, channel: str, first_seen: Optional[str] = None) -> List[Dict]:
    """
    Parses a single raw Telegram message and extracts:
    - Airdrop name
    - Airdrop link
    - Channel name
    - Raw text

    Returns: List[Dict] — one campaign per link
    """
    links = re.findall(r"https?://\S+", raw)

    campaigns = []
    for link in links:
        # Extract airdrop name heuristically from the domain
        name_match = re.search(r"(?:https?://)?(?:www\.)?([\w\-\.]+)\.\w+", link)
        name = name_match.group(1).capitalize() if name_match else "Unknown"

        campaigns.append({
            "airdrop_name": name,
            "link": link,
            "scan_type": "initial" if first_seen == "initial" else "cron",
            "channel": channel,
            "raw_text": raw.strip()
        })

    return campaigns

