#!/usr/bin/env python3
"""
解析 ias-api-doc 风格的 Markdown 接口文档，提取接口定义 + 枚举候选。
输出 JSON，供 bridge_client 写入 ApiCat。

用法：
    python parse_markdown_docs.py <markdown_file> [--project-id N]

输出格式：
{
  "project_id": N,
  "requests": [
    {
      "name": "获取推荐",
      "method": "GET",
      "url": "/apm/intl/recommend/migration",
      "description": "...",
      "params": [{"key":"gpId","value":"","enabled":true,"type":"string","description":"设备 GAID"}],
      "enum_candidates": {"status": [{"value":"0","label":"待支付"},{"value":"1","label":"已支付"}]}
    }
  ]
}
"""
import argparse
import json
import re
import sys
from pathlib import Path


def parse_markdown(filepath: str) -> list[dict]:
    """解析一个 markdown 文件中的接口定义"""
    text = Path(filepath).read_text(encoding="utf-8")
    requests = []

    # 匹配 #### 标题（接口名）及其后的内容
    # 格式：#### 接口中文名 /path 或 #### 接口中文名（无 path 时从请求块提取）
    sections = re.split(r'^####\s+', text, flags=re.MULTILINE)

    for section in sections[1:]:  # 跳过第一个（标题前的内容）
        lines = section.strip().split('\n')
        title_line = lines[0].strip()

        # 从标题提取名称和路径
        # 形如 "获取换机迁移推荐" 或 "获取推荐 /apm/intl/recommend"
        path_match = re.search(r'(/[a-zA-Z0-9/_\-{}]+)', title_line)
        name = re.sub(r'\s*/[a-zA-Z0-9/_\-{}]+.*$', '', title_line).strip()
        url = path_match.group(1) if path_match else ''

        # 从请求块提取 method 和 url
        method = 'GET'
        for line in lines:
            method_match = re.search(r'`(GET|POST|PUT|DELETE|PATCH)`', line)
            if method_match:
                method = method_match.group(1)
            # 请求 URL 行：- https://... 或 - 线上域名：https://...
            url_match = re.search(r'https?://[^\s]+(/(?:apm|api|intl)[^\s]*)', line)
            if url_match and not url:
                url = url_match.group(1)

        # 提取参数表
        params = []
        in_param_table = False
        for line in lines:
            if re.match(r'\|\s*参数\s*\|', line) or re.match(r'\|\s*参数名\s*\|', line):
                in_param_table = True
                continue
            if in_param_table:
                if not line.strip().startswith('|'):
                    in_param_table = False
                    continue
                if re.match(r'\|[\s\-|]+\|', line):  # 分隔行
                    continue
                cells = [c.strip() for c in line.strip().strip('|').split('|')]
                if len(cells) >= 3:
                    pname = cells[0].strip('`')
                    ptype = cells[1].strip('`') if len(cells) > 1 else 'string'
                    prequired = cells[2].strip() if len(cells) > 2 else ''
                    pdesc = cells[3].strip() if len(cells) > 3 else ''
                    params.append({
                        'key': pname,
                        'value': '',
                        'enabled': True,
                        'type': ptype,
                        'description': pdesc,
                        'required': prequired in ('是', '必填'),
                    })

        # 枚举候选：从描述中提取 `0`=xx、`1`=yy 模式
        enum_candidates = {}
        for p in params:
            desc = p.get('description', '')
            pairs = re.findall(r'`(\w+)`\s*=\s*([^\s,、;；]+)', desc)
            if len(pairs) >= 2:
                enum_candidates[p['key']] = [
                    {'value': v, 'label': label} for v, label in pairs
                ]

        if name:
            requests.append({
                'name': name,
                'method': method,
                'url': url,
                'params': params,
                'enum_candidates': enum_candidates,
            })

    return requests


def main():
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
