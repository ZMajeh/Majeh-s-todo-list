path = '/home/majeh/git/Majeh-s-todo-list/plan.md'
with open(path, 'r') as f:
    lines = f.readlines()
for i in range(283, min(312, len(lines))):
    print(f'{i+1}: {repr(lines[i])}')