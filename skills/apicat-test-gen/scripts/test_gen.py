#!/usr/bin/env python3
"""
ApiCat AI 测试用例写入脚本。

经 HTTP Bridge 写入（与 apicat-lib 铁律一致，不直连 SQLite）：
建/复用「🤖 AI 测试用例」目录 → 逐条 POST /create_test_case。

用法：
    python test_gen.py --project <项目ID> --file <用例JSON文件>

用例 JSON 为数组，元素字段（snake_case）：
    name / description / method / url / headers / params /
    body_type / body / assertions / request_id

幂等：request_id 非空时，先查该接口已有用例，同 name 跳过。
"""
import argparse
import json
import sys
from pathlib import Path

# bridge_client 位于同级技能 apicat-lib 的 scripts/ 下。
# parents[2] 从 skills/<技能>/scripts/ 上溯到 skills/ 目录 —— 仓库内与安装后
# （junction 或复制）都成立，故不要改成依赖仓库布局的路径。
_SKILLS_DIR = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(_SKILLS_DIR / "apicat-lib" / "scripts"))
from bridge_client import ApiCatBridge  # noqa: E402

AI_COLLECTION_NAME = "🤖 AI 测试用例"
AI_SOURCE = "ai_generated"


def get_or_create_ai_collection(bridge: ApiCatBridge, project_id: int) -> int:
    """复用同名目录，不存在则创建。"""
    for coll in bridge.get("/list_collections", {"project_id": project_id}):
        if coll.get("name") == AI_COLLECTION_NAME:
            return coll["id"]
    created = bridge.post("/create_collection", {
        "projectId": project_id,
        "parentId": None,
        "name": AI_COLLECTION_NAME,
    })
    return created["id"]


def _json_field(value, default):
    """入参里 headers/params/assertions 允许是数组或已是 JSON 字符串，统一成字符串。"""
    if value is None:
        return json.dumps(default, ensure_ascii=False)
    if isinstance(value, str):
        return value
    return json.dumps(value, ensure_ascii=False)


def _to_bridge_payload(case: dict, collection_id: int) -> dict:
    """把输入 JSON 的 snake_case 字段映射成 Bridge 的 camelCase 载荷。"""
    return {
        "requestId": case.get("request_id"),
        "collectionId": collection_id,
        "name": case.get("name", "Generated Test Case"),
        "description": case.get("description", ""),
        "source": AI_SOURCE,
        "method": case.get("method"),
        "url": case.get("url"),
        "headers": _json_field(case.get("headers"), []),
        "params": _json_field(case.get("params"), []),
        "bodyType": case.get("body_type"),
        "body": case.get("body", ""),
        # 必须显式传：后端缺省兜底是 happy_path，漏传会把 7 种类型全标成 happy_path
        "caseType": case.get("case_type"),
        "assertions": _json_field(case.get("assertions"), []),
    }


def _existing_names(bridge: ApiCatBridge, request_id: int) -> set[str]:
    """取某接口已有用例名，供幂等跳过。"""
    return {c["name"] for c in bridge.get("/list_test_cases", {"request_id": request_id})}


def write_test_cases(bridge: ApiCatBridge, project_id: int, cases_file: str) -> None:
    path = Path(cases_file)
    if not path.exists():
        print(f"Error: Cases file not found: {cases_file}")
        sys.exit(1)

    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        print(f"Error parsing JSON: {e}")
        sys.exit(1)
    if not isinstance(data, list):
        print("Error: cases file must be a JSON array.")
        sys.exit(1)

    collection_id = get_or_create_ai_collection(bridge, project_id)

    # 按 request_id 缓存已有用例名，避免逐条重复拉取
    name_cache: dict[int, set[str]] = {}
    written = skipped = 0

    for case in data:
        rid = case.get("request_id")
        name = case.get("name", "Generated Test Case")
        if rid is not None:
            if rid not in name_cache:
                name_cache[rid] = _existing_names(bridge, rid)
            if name in name_cache[rid]:
                skipped += 1
                continue
            name_cache[rid].add(name)

        bridge.post("/create_test_case", _to_bridge_payload(case, collection_id))
        written += 1

    print(f"Successfully generated and inserted {written} test cases"
          + (f" (skipped {skipped} existing)." if skipped else "."))


def main() -> None:
    # Windows 下 Python 默认按 locale(GBK) 写 stdout/stderr，中文会乱码
    for _stream in (sys.stdout, sys.stderr):
        if hasattr(_stream, "reconfigure"):
            _stream.reconfigure(encoding="utf-8")

    parser = argparse.ArgumentParser(description="ApiCat AI Test Case Generator")
    parser.add_argument("--project", type=int, required=True, help="ApiCat Project ID")
    parser.add_argument("--file", type=str, required=True, help="JSON file containing test cases definition")
    args = parser.parse_args()

    try:
        bridge = ApiCatBridge()
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    write_test_cases(bridge, args.project, args.file)


if __name__ == "__main__":
    main()
