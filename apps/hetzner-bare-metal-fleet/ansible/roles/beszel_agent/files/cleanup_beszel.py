import re

with open('/root/docker-compose.yml', 'r') as f:
    lines = f.readlines()

result = []
skip_until_indent = None

for i, line in enumerate(lines):
    # Skip any Ansible marker comments
    if 'ANSIBLE MANAGED BLOCK' in line:
        continue

    # If we're currently skipping lines
    if skip_until_indent is not None:
        if line.strip() == '':
            # Skip blank lines while in skip mode
            continue

        current_indent = len(line) - len(line.lstrip())

        # If we find a line at or before the indent level, stop skipping
        if current_indent <= skip_until_indent:
            skip_until_indent = None
            # Check if this line starts ANOTHER beszel-agent service
            if re.match(r'^(\s+)beszel-agent:\s*$', line):
                skip_until_indent = len(line) - len(line.lstrip())
                continue
            # Otherwise fall through to append this line
        else:
            # Still deeper than target indent, keep skipping
            continue

    # Check if this line starts a beszel-agent service
    match = re.match(r'^(\s+)beszel-agent:\s*$', line)
    if match:
        skip_until_indent = len(match.group(1))
        continue

    # Check for orphaned beszel-agent property
    if re.match(r'^\s+image:\s+henrygd/beszel-agent', line):
        has_service_name = False
        for j in range(i - 1, -1, -1):
            prev_line = lines[j]
            if prev_line.strip() == '':
                continue
            if re.match(r'^  \w+:\s*$', prev_line):
                if 'beszel-agent:' in prev_line:
                    has_service_name = True
                break
            elif len(prev_line) - len(prev_line.lstrip()) > 2:
                break
            else:
                break

        if not has_service_name:
            property_indent = len(line) - len(line.lstrip())
            skip_until_indent = property_indent - 2
            continue

    if skip_until_indent is None:
        result.append(line)

with open('/root/docker-compose.yml', 'w') as f:
    f.writelines(result)
