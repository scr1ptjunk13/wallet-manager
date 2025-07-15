import re
import yaml
from pathlib import Path

def load_config():
    """Load configuration from config.yaml."""
    config_path = Path(__file__).parent / "config.yaml"
    with open(config_path, "r") as f:
        return yaml.safe_load(f)

def load_subreddits():
    """Load list of subreddits from subs.txt."""
    subs_path = Path(__file__).parent / "subs.txt"
    with open(subs_path, "r") as f:
        return [line.strip() for line in f if line.strip()]

def clean_text(text):
    """Clean text by removing markdown, URLs, and extra whitespace."""
    if not text:
        return ""
    # Remove URLs
    text = re.sub(r'http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\\(\\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+', '', text)
    # Remove markdown symbols (e.g., *, #, >)
    text = re.sub(r'[\*\#\>]+', '', text)
    # Normalize whitespace
    text = ' '.join(text.strip().split())
    return text

def extract_chains(text):
    """Extract blockchain names from text."""
    chains = ["Ethereum", "Arbitrum", "Polygon", "BSC", "Solana", "Avalanche"]
    found = [chain for chain in chains if chain.lower() in text.lower()]
    return found if found else ["Unknown"]

def extract_tokens(text):
    """Extract token names from text."""
    tokens = ["ETH", "USDT", "USDC", "DAI", "BNB"]
    found = [token for token in tokens if token in text.upper()]
    return found if found else ["Unknown"]

def extract_requirements(text):
    """Extract airdrop requirements from text."""
    keywords = ["connect wallet", "provide liquidity", "daily activity", "community", "stake", "swap"]
    found = [req for req in keywords if req in text.lower()]
    return found if found else ["Unknown"]
