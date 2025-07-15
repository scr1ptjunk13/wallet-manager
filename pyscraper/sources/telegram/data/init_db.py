import sqlite3
import json
from datetime import datetime

DB_PATH = "data/enriched_campaigns.db"

def init_db():
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute("""
    CREATE TABLE IF NOT EXISTS campaigns (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        airdrop_name TEXT,
        link TEXT,
        telegram_timestamp TEXT,
        scan_type TEXT,
        channel TEXT,
        raw_text TEXT,
        scraped_at TEXT,
        platform TEXT,
        reward TEXT,
        reward_type TEXT,
        deadline TEXT,
        requirements TEXT,
        claimable BOOLEAN,
        tokens_per_action TEXT
    );
    """)
    conn.commit()
    conn.close()


def insert_campaign(data):
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()

    # Convert requirements list to string if needed
    requirements_str = (
        json.dumps(data["requirements"]) if isinstance(data.get("requirements"), list)
        else data.get("requirements", "")
    )

    c.execute("""
    INSERT INTO campaigns (
        airdrop_name, link, telegram_timestamp, scan_type, channel, raw_text, scraped_at,
        platform, reward, reward_type, deadline, requirements, claimable, tokens_per_action
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, (
        data.get("airdrop_name"),
        data.get("link"),
        data.get("telegram_timestamp"),
        data.get("scan_type"),
        data.get("channel"),
        data.get("raw_text"),
        data.get("fetched_at", datetime.utcnow().isoformat()),
        data.get("platform"),
        data.get("reward"),
        data.get("reward_type"),
        data.get("deadline"),
        requirements_str,
        data.get("claimable", False),
        data.get("tokens_per_action")
    ))

    conn.commit()
    conn.close()


if __name__ == "__main__":
    init_db()

