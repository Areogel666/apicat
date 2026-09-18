#!/usr/bin/env python3
"""
解析 ias-api-doc 风格的 Markdown 接口文档，提取接口定义 + 枚举候选。

位于 apicat-edit（数据编辑技能）。**只解析，不写库** —— 输出 JSON 后，
由调用方按 apicat-edit/SKILL.md「从文档批量导入接口」的流程逐步落库。

用法：
    python parse_markdown_docs.py <markdown_file> [--project-id N]

输出格式：
{
  "project_id": N,
  "requests": [
    {
      "name": "获取用户信息",
      "method": "GET",
      "url": "/api/user/info",
      "params": [{"key":"userId","value":"","enabled":true,"type":"string","description":"用户 ID","required":true}],
      "enum_candidates": {"status": [{"value":"0","label":"待支付"},{"value":"1","label":"已支付"}]}
    }
  ]
}

注：当前版本不输出小节级 description 字段（解析器未实现该提取）。
"""
import argparse
import json
import re
import sys
from pathlib import Path

# —— 解析用正则，集中在此便于统一维护 ——
_SECTION_SPLIT_RE = re.compile(r'^####\s+', re.MULTILINE)
_TITLE_PATH = r'/[a-zA-Z0-9/_\-{}]+'
_TITLE_PATH_RE = re.compile(rf'({_TITLE_PATH})')
_TITLE_NAME_STRIP_RE = re.compile(rf'\s*{_TITLE_PATH}.*$')
_METHOD_RE = re.compile(r'`(GET|POST|PUT|DELETE|PATCH)`')
_URL_RE = re.compile(r'https?://[^\s]+(/(?:apm|api|intl)[^\s]*)')
_TABLE_HEADER_RE = re.compile(r'\|\s*参数\s*\|')
_TABLE_HEADER_NAME_RE = re.compile(r'\|\s*参数名\s*\|')
_TABLE_SEP_RE = re.compile(r'\|[\s\-|]+\|')
_ENUM_PAIR_RE = re.compile(r'`(\w+)`\s*=\s*([^\s,、;；]+)')


def _parse_title(title_line: str) -> tuple[str, str]:
    """从标题行提取接口名与路径。

    形如 "获取用户信息" 或 "获取用户信息 /api/user/info"。
    路径可能为空串（后续由 _scan_method_url 从请求块补全）。
    """
    path_match = _TITLE_PATH_RE.search(title_line)
    name = _TITLE_NAME_STRIP_RE.sub('', title_line).strip()
    url = path_match.group(1) if path_match else ''
    return name, url


def _scan_method_url(lines: list[str], url: str) -> tuple[str, str]:
    """扫描小节正文补全 method 与 url。

    method 取**首次**命中（请求方式行通常在参数表之前，可避免参数描述里的
    `GET`/`POST` 枚举示例覆盖真实 method）；url 仅在标题未解析出路径时才从请求行提取。
    """
    method = None
    for line in lines:
        if method is None:
            method_match = _METHOD_RE.search(line)
            if method_match:
                method = method_match.group(1)
        # 请求 URL 行：- https://... 或 - 线上域名：https://...
        url_match = _URL_RE.search(line)
        if url_match and not url:
            url = url_match.group(1)
    return method or 'GET', url


def _parse_param_table(lines: list[str]) -> list[dict]:
    """解析参数表状态机：遇表头开启，遇非 | 行关闭，跳过分隔行。"""
    params = []
    in_param_table = False
    for line in lines:
        if _TABLE_HEADER_RE.match(line) or _TABLE_HEADER_NAME_RE.match(line):
            in_param_table = True
            continue
        if in_param_table:
            if not line.strip().startswith('|'):
                in_param_table = False
                continue
            if _TABLE_SEP_RE.match(line):  # 分隔行
                continue
            cells = [c.strip() for c in line.strip().strip('|').split('|')]
            # 外层已保证 >= 3 列，故第 1/2 列无需再判空；第 4 列（描述）可能不存在
            if len(cells) >= 3:
                params.append({
                    'key': cells[0].strip('`'),
                    'value': '',
                    'enabled': True,
                    'type': cells[1].strip('`'),
                    'description': cells[3].strip() if len(cells) > 3 else '',
                    'required': cells[2].strip() in ('是', '必填'),
                })
    return params


def _extract_enum_candidates(params: list[dict]) -> dict:
    """从参数描述中提取 `0`=xx、`1`=yy 形式的枚举候选（少于 2 对不产出）。"""
    enum_candidates = {}
    for p in params:
        desc = p.get('description', '')
        pairs = _ENUM_PAIR_RE.findall(desc)
        if len(pairs) >= 2:
            enum_candidates[p['key']] = [
                {'value': v, 'label': label} for v, label in pairs
            ]
    return enum_candidates


def _parse_section(lines: list[str]) -> dict | None:
    """解析单个 #### 小节；标题取不到 name 时返回 None。"""
    name, url = _parse_title(lines[0].strip())
    if not name:
        return None
    method, url = _scan_method_url(lines, url)
    params = _parse_param_table(lines)
    return {
        'name': name,
        'method': method,
        'url': url,
        'params': params,
        'enum_candidates': _extract_enum_candidates(params),
    }


def parse_markdown(filepath: str) -> list[dict]:
    """解析一个 markdown 文件中的接口定义"""
    text = Path(filepath).read_text(encoding='utf-8')
    # 匹配 #### 标题（接口名）及其后的内容
    # 格式：#### 接口中文名 /path 或 #### 接口中文名（无 path 时从请求块提取）
    sections = _SECTION_SPLIT_RE.split(text)

    requests = []
    for section in sections[1:]:  # 跳过第一个（标题前的内容）
        lines = section.strip().split('\n')
        parsed = _parse_section(lines)
        if parsed is not None:
            requests.append(parsed)
    return requests


def main() -> None:
    # Windows 下 Python 默认按 locale(GBK) 写 stdout/stderr，会把中文接口名/描述输出成乱码，
    # 强制 UTF-8 保证调用方（AI/终端）能正确读取
    for _stream in (sys.stdout, sys.stderr):
        if hasattr(_stream, "reconfigure"):
            _stream.reconfigure(encoding="utf-8")

    parser = argparse.ArgumentParser(description='解析 Markdown 接口文档')
    parser.add_argument('filepath', help='Markdown 文件路径')
    parser.add_argument('--project-id', type=int, help='目标项目 ID')
    args = parser.parse_args()

    if not Path(args.filepath).exists():
        print(f"文件不存在: {args.filepath}", file=sys.stderr)
        sys.exit(1)

    requests = parse_markdown(args.filepath)
    output = {
        'project_id': args.project_id,
        'source_file': str(args.filepath),
        'requests': requests,
    }
    print(json.dumps(output, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
