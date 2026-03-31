import sqlite3
import json
import sys
import os
import argparse

def get_db_path():
    appdata = os.getenv('APPDATA')
    if not appdata:
        print("Error: APPDATA environment variable not found.")
        sys.exit(1)
    db_path = os.path.join(appdata, 'com.apicat.app', 'apicat.db')
    if not os.path.exists(db_path):
        print(f"Error: ApiCat database not found at {db_path}.")
        sys.exit(1)
    return db_path

def get_or_create_ai_collection(conn, project_id):
    cursor = conn.cursor()
    cursor.execute("SELECT id FROM collections WHERE project_id = ? AND name = ?", (project_id, "🤖 AI 测试用例"))
    row = cursor.fetchone()
    if row:
        return row[0]
    cursor.execute(
        "INSERT INTO collections (project_id, name, sort_order) VALUES (?, ?, ?)",
        (project_id, "🤖 AI 测试用例", 999)
    )
    return cursor.lastrowid

def write_test_cases(project_id, cases_file):
    if not os.path.exists(cases_file):
        print(f"Error: Cases file not found: {cases_file}")
        sys.exit(1)

    try:
        with open(cases_file, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except Exception as e:
        print(f"Error parsing JSON: {e}")
        sys.exit(1)
    
    db_path = get_db_path()
    conn = sqlite3.connect(db_path)
    
    try:
        collection_id = get_or_create_ai_collection(conn, project_id)
        cursor = conn.cursor()
        count = 0
        for case in data:
            name = case.get("name", "Generated Test Case")
            description = case.get("description", "")
            method = case.get("method")
            url = case.get("url")
            headers = json.dumps(case.get("headers", []), ensure_ascii=False)
            params = json.dumps(case.get("params", []), ensure_ascii=False)
            body_type = case.get("body_type")
            body = case.get("body", "")
            assertions = json.dumps(case.get("assertions", []), ensure_ascii=False)
            request_id = case.get("request_id")

            cursor.execute("""
                INSERT INTO test_cases (
                    collection_id, request_id, name, description, source,
                    method, url, headers, params, body_type, body, assertions,
                    starred, enabled
                ) VALUES (?, ?, ?, ?, 'ai_generated', ?, ?, ?, ?, ?, ?, ?, 0, 1)
            """, (
                collection_id, request_id, name, description,
                method, url, headers, params, body_type, body, assertions
            ))
            count += 1
            
        conn.commit()
        print(f"Successfully generated and inserted {count} test cases.")
    except Exception as e:
        conn.rollback()
        print(f"Database error: {e}")
        sys.exit(1)
    finally:
        conn.close()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ApiCat AI Test Case Generator")
    parser.add_argument("--project", type=int, required=True, help="ApiCat Project ID")
    parser.add_argument("--file", type=str, required=True, help="JSON file containing test cases definition")
    args = parser.parse_args()
    write_test_cases(args.project, args.file)
