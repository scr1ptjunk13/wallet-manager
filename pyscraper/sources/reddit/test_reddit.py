import praw
from utils import load_config

def test_reddit_connection():
    """Test Reddit API connection"""
    try:
        config = load_config()
        reddit = praw.Reddit(
            client_id=config["reddit"]["client_id"],
            client_secret=config["reddit"]["client_secret"],
            user_agent=config["reddit"]["user_agent"]
        )
        
        # Test by getting a single post from a popular subreddit
        subreddit = reddit.subreddit("python")
        post = next(subreddit.hot(limit=1))
        
        print(f"✅ Connection successful!")
        print(f"Test post title: {post.title[:50]}...")
        print(f"Reddit user: {reddit.user.me()}")
        
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        print("\nTroubleshooting:")
        print("1. Make sure your Reddit app is type 'script' (not 'web app')")
        print("2. Double-check your client_id and client_secret")
        print("3. Verify your Reddit account has API access")

if __name__ == "__main__":
    test_reddit_connection()
