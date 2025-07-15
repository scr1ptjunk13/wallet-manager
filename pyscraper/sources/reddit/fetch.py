import praw
import sqlite3
from pathlib import Path
import sys
import os

# Add the current directory to Python path
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from utils import load_config, load_subreddits
from parser import parse_post

def init_db():
    """Initialize SQLite database."""
    db_path = Path(__file__).parent / "results.db"
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS airdrops (
            id TEXT PRIMARY KEY,
            data TEXT
        )
    """)
    conn.commit()
    return conn, cursor

def save_campaign(cursor, conn, campaign):
    """Save campaign data to SQLite database."""
    import json
    cursor.execute(
        "INSERT OR REPLACE INTO airdrops (id, data) VALUES (?, ?)",
        (campaign["source"]["reddit"]["post_id"], json.dumps(campaign))
    )
    conn.commit()

def fetch_reddit_data():
    """Fetch and process Reddit posts."""
    config = load_config()
    reddit = praw.Reddit(
        client_id=config["reddit"]["client_id"],
        client_secret=config["reddit"]["client_secret"],
        user_agent=config["reddit"]["user_agent"]
    )
    subreddits = load_subreddits()
    conn, cursor = init_db()

    for subreddit_name in subreddits:
        try:
            subreddit = reddit.subreddit(subreddit_name)
            for post in subreddit.hot(limit=config["scraper"]["max_posts"]):
                campaign = parse_post(post)
                if campaign:
                    save_campaign(cursor, conn, campaign)
        except Exception as e:
            print(f"Error processing r/{subreddit_name}: {e}")

    conn.close()

if __name__ == "__main__":
    fetch_reddit_data()
