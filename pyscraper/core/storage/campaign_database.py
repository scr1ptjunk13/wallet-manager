import sqlite3
import os

class CampaignDatabase:
    def __init__(self, db_path="galxe_campaigns.db"):
        self.conn = sqlite3.connect(db_path)
        self.conn.row_factory = sqlite3.Row
        self.cursor = self.conn.cursor()

    def get_campaigns(self, limit=10, status=None):
        query = "SELECT * FROM campaigns"
        params = []

        if status:
            query += " WHERE status = ?"
            params.append(status)

        query += " ORDER BY scraped_at DESC LIMIT ?"
        params.append(limit)

        try:
            self.cursor.execute(query, tuple(params))
            rows = self.cursor.fetchall()
            return [dict(row) for row in rows]
        except Exception as e:
            print(f"DB error: {e}")
            return []

