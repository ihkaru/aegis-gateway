#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""
Aegis Gateway: Hermetic SQLite MCP Server
Provides real, persistent SQLite database tools over standard JSON-RPC 2.0 stdio.
"""

import json
import os
import sqlite3
import sys

DB_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "data"))
os.makedirs(DB_DIR, exist_ok=True)
DB_PATH = os.path.join(DB_DIR, "aegis_local.db")


def init_db():
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    cur.execute("""
        CREATE TABLE IF NOT EXISTS audit_ledger (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            action TEXT NOT NULL,
            target TEXT NOT NULL,
            payload_digest TEXT NOT NULL,
            status TEXT NOT NULL
        )
    """)
    cur.execute("""
        CREATE TABLE IF NOT EXISTS customer_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            customer_ref TEXT UNIQUE NOT NULL,
            tier TEXT NOT NULL,
            balance REAL DEFAULT 0.0,
            active INTEGER DEFAULT 1
        )
    """)
    cur.execute("SELECT COUNT(*) FROM customer_accounts")
    if cur.fetchone()[0] == 0:
        cur.executemany(
            "INSERT INTO customer_accounts (customer_ref, tier, balance, active) VALUES (?, ?, ?, ?)",
            [
                ("CUST-ENT-101", "Enterprise-Strict", 50000.0, 1),
                ("CUST-CORP-202", "Corporate-Hybrid", 12500.5, 1),
                ("CUST-DEV-303", "Developer-Standard", 250.0, 1),
            ],
        )
    conn.commit()
    conn.close()


def execute_query(query: str):
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    try:
        cur.execute(query)
        q_upper = query.strip().upper()
        if q_upper.startswith(("SELECT", "PRAGMA", "EXPLAIN")):
            cols = [d[0] for d in cur.description] if cur.description else []
            rows = [dict(zip(cols, r)) for r in cur.fetchall()]
            conn.close()
            return {"status": "SUCCESS", "type": "SELECT", "row_count": len(rows), "rows": rows}
        else:
            conn.commit()
            affected = cur.rowcount
            conn.close()
            return {"status": "SUCCESS", "type": "MUTATION", "rows_affected": affected}
    except Exception as e:
        conn.close()
        return {"status": "ERROR", "error": str(e)}


def main():
    init_db()
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except Exception:
            continue

        req_id = req.get("id")
        method = req.get("method")
        params = req.get("params", {})

        if method == "tools/list":
            resp = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": [
                        {
                            "name": "sqlite_query",
                            "description": "Execute live SQL query on local persistent SQLite database",
                            "inputSchema": {
                                "type": "object",
                                "properties": {"query": {"type": "string"}},
                                "required": ["query"],
                            },
                        },
                        {
                            "name": "sqlite_schema",
                            "description": "Inspect live table schema in local SQLite database",
                            "inputSchema": {
                                "type": "object",
                                "properties": {"table": {"type": "string"}},
                            },
                        },
                    ]
                },
            }
        elif method == "tools/call":
            tool_name = params.get("name", "")
            args = params.get("arguments", {})
            if tool_name == "sqlite_query":
                sql = args.get("query", "SELECT * FROM customer_accounts LIMIT 5")
                data = execute_query(sql)
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [{"type": "text", "text": json.dumps(data)}],
                        "isError": data.get("status") == "ERROR",
                        "data": data,
                    },
                }
            elif tool_name == "sqlite_schema":
                table = args.get("table", "customer_accounts")
                data = execute_query(f"PRAGMA table_info({table})")
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [{"type": "text", "text": json.dumps(data)}],
                        "isError": data.get("status") == "ERROR",
                        "data": data,
                    },
                }
            else:
                resp = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {"code": -32601, "message": f"Method {tool_name} not found"},
                }
        else:
            resp = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {"status": "ready"},
            }

        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
