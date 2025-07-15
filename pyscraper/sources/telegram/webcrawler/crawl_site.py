import requests
from bs4 import BeautifulSoup
from urllib.parse import urljoin
import re


def clean_text(text):
    return re.sub(r"\s+", " ", text.strip())

def extract_info_from_airdrop_page(url):
    """
    Given a URL of an airdrop campaign page, extract key info for automation.
    """
    try:
        headers = {
            "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115 Safari/537.36"
        }
        resp = requests.get(url, headers=headers, timeout=10)
        if resp.status_code != 200:
            print(f"Failed to fetch page: {url} => Status: {resp.status_code}")
            return None

        soup = BeautifulSoup(resp.text, "html.parser")

        # Fallback text if structure isn't clean
        full_text = clean_text(soup.get_text(" "))

        # Heuristics based extraction
        data = {
            "project_name": None,
            "description": None,
            "platform": None,
            "token": None,
            "value_estimate": None,
            "claim_url": url,
            "deadline": None,
            "eligibility": None,
            "requirements": [],
            "how_to": None,
            "snapshot_date": None
        }

        # Project name from meta or heading
        title_tag = soup.find("title")
        if title_tag:
            data["project_name"] = clean_text(title_tag.text.split("Airdrop")[0])

        # Description from intro
        desc_match = re.search(r"What is (.*?)\? (.*?)Caldera Airdrop Details", full_text)
        if desc_match:
            data["description"] = clean_text(desc_match.group(2))

        # Token info
        token_match = re.search(r"token\s+([A-Z]{2,5})", full_text, re.IGNORECASE)
        if token_match:
            data["token"] = token_match.group(1).upper()

        # Value estimate
        value_match = re.search(r"Estimated Value\s*\n?\s*(.*?)\s*\n", full_text)
        if value_match:
            data["value_estimate"] = clean_text(value_match.group(1))

        # Deadline
        deadline_match = re.search(r"(Pre-claim Deadline|Ends on):?\s*(\w+ \d{1,2},? \d{4})", full_text)
        if deadline_match:
            data["deadline"] = clean_text(deadline_match.group(2))

        # Eligibility
        elig_match = re.search(r"Eligibility Categories(.*?)Caldera Username Registration Launch", full_text, re.DOTALL)
        if elig_match:
            data["eligibility"] = clean_text(elig_match.group(1))

        # Snapshot date
        snap_match = re.search(r"Snapshot Date:?\s*(.*?)\s*\n", full_text)
        if snap_match:
            data["snapshot_date"] = clean_text(snap_match.group(1))

        # Platform (might be mentioned with "platform: <value>")
        plat_match = re.search(r"Platform:?\s*(.*?)\s*\n", full_text)
        if plat_match:
            data["platform"] = clean_text(plat_match.group(1))

        # Tasks
        howto_match = re.search(r"Step-by-Step Guide:(.*?)Important Deadline Information", full_text, re.DOTALL)
        if howto_match:
            data["how_to"] = clean_text(howto_match.group(1))
            steps = re.findall(r"\d\.?\)?\s+([^\n]+)", data["how_to"])
            data["requirements"] = [clean_text(s) for s in steps if len(s) > 3]

        return data

    except Exception as e:
        print(f"Error while crawling {url}: {e}")
        return None


if __name__ == "__main__":
    test_url = "https://airdrops.io/caldera"
    result = extract_info_from_airdrop_page(test_url)
    import json
    print(json.dumps(result, indent=2))

