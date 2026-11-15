path = '/home/majeh/git/Majeh-s-todo-list/plan.md'
with open(path, 'r') as f:
    c = f.read()

old = """### Desktop

- Wider content layout
- Expanded task view
- Optimized use of available space

#### Desktop Behavior"""

new = """### Desktop - Three-Column Layout

The desktop version follows a three-panel layout inspired by Notion + TickTick + Microsoft To Do:

#### Left Sidebar (Navigation)

- Task list navigation
- Completed tasks view
- Sync status badge
- Import/Export controls
- Settings button
- Google account connection indicator (Google Drive)
- Modern glassy sidebar styling

#### Center Panel (Main Task View)

- Date navigation bar: < Nov 14, 2026 >
- Search/filtering bar
- Nested task hierarchy tree
- Deep nesting support (Build SaaS → Backend → OAuth)
- Drag-drop sorting support
- Task completion checkboxes

#### Right Details Panel

- Selected task information
- Created Date
- Due Date
- Priority selector
- Notes field
- Tags
- Activity history
- Sync state

#### Design Style

- Blue accent colors matching Majeh's TODO List branding
- Modern glassy sidebar
- Light and dark mode ready styling
- Sync status badge and Google Drive connection indicator

#### Desktop Behavior"""

if old in c:
    c = c.replace(old, new)
    with open(path, 'w') as f:
        f.write(c)
    print('Success: Desktop section updated')
else:
    print('Not found - checking')
    for i, line in enumerate(c.split('\n'), 1):
        if '### Desktop' in line:
            print(f'Line {i}: {repr(line)}')