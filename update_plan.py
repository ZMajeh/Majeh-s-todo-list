path = '/home/majeh/git/Majeh-s-todo-list/plan.md'
with open(path, 'r') as f:
    c = f.read()
old = 'Switching between screen sizes should adjust the UI seamlessly without requiring a page refresh.\n---\n\n# Technical Goals'
new = '''#### Desktop Behavior
- Right-side detail panel stays open while navigating tasks
- Deep task nesting (Build SaaS → Backend → OAuth → API)
- Click task to view details in right panel
- Date navigation changes center panel tasks

Switching between screen sizes should adjust the UI seamlessly without requiring a page refresh.
---

# Technical Goals'''
if old in c:
    c = c.replace(old, new)
    with open(path, 'w') as f:
        f.write(c)
    print('Updated Desktop Behavior section')
else:
    print('Pattern not found')