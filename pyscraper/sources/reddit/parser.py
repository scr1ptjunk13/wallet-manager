from utils import clean_text, extract_chains, extract_tokens, extract_requirements, load_config
import re
from datetime import datetime

def parse_post(post):
    """Parse a Reddit post into an airdrop data structure."""
    title = clean_text(post.title)
    body = clean_text(post.selftext)
    content = title + " " + body
    config = load_config()
    keywords = config["keywords"]

    # Check if post is relevant (contains any airdrop-related keywords)
    if not any(keyword.lower() in content.lower() for keyword in keywords):
        return None

    # Basic scoring based on upvotes and comments
    score = min(post.score / 1000.0, 1.0) + min(len(post.comments) / 100.0, 0.5)  # Fixed the incomplete line
    
    return {
        "airdrop_name": extract_name(title),
        "category": "DeFi" if "defi" in content.lower() else "Unknown",
        "chain": extract_chains(content),
        "project_link": extract_link(content),
        "airdrop_link": extract_link(content),
        "requirements": extract_requirements(content),
        "required_tokens": extract_tokens(content),
        "wallet_tags": ["defi_active", "multi_chain"] if len(extract_chains(content)) > 1 else ["defi_active"],
        "deadline": None,
        "estimated_reward": "Speculative - No confirmed amount",
        "effort_level": "medium",
        "risk_level": "medium",
        "task_type": infer_task_types(extract_requirements(content)),
        "automatable": True,
        "additional_notes": f"Reddit post from r/{post.subreddit.display_name} (Score: {post.score})",
        "source": {
            "reddit": {
                "post_id": post.id,
                "subreddit": post.subreddit.display_name,
                "url": post.url
            }
        },
        "fetched_at": datetime.utcfromtimestamp(post.created_utc).isoformat() + "Z",
        "confidence_score": score,
        "airdrop_status": "speculative"
    }

def extract_name(title):
    """Extract airdrop/project name from title."""
    # Simple heuristic: take the first capitalized phrase
    match = re.search(r'\b[A-Z][a-zA-Z]*\b', title)
    return match.group(0) if match else "Unknown"

def extract_link(text):
    """Extract project/airdrop link (placeholder, as URLs are removed in clean_text)."""
    return "Unknown"  # Implement actual link extraction if needed

def infer_task_types(requirements):
    """Map requirements to task types."""
    mapping = {
        "connect wallet": "WalletConnection",
        "provide liquidity": "LiquidityProvision",
        "daily activity": "DailyActivity",
        "community": "CommunityEngagement",
        "stake": "Staking",
        "swap": "Swap"
    }
    return [mapping.get(req, "Unknown") for req in requirements]
