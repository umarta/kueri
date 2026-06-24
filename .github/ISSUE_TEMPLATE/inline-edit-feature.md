---
name: Inline Edit Feature
about: Enable inline editing of record values directly in the grid
title: "Inline-edit: modify record values directly in grid"
labels: enhancement, area:grid
---

## Description

Add inline-edit mode to allow users to modify record values directly in the grid without opening a side panel. This feature should be configurable in the settings, with options to toggle between:

1. **Inline-edit mode**: Edit cells directly in the grid
2. **Side-bar detail mode** (rowdetail.svelte): Open a side panel for detailed row editing

## Acceptance Criteria

- [ ] Users can toggle inline-edit mode on/off in settings
- [ ] When enabled, double-click or single-click on a cell to edit its value inline
- [ ] Pressing Enter saves the change, Escape cancels
- [ ] Tab navigation between cells in edit mode
- [ ] Support for all data types (text, number, date, FK, etc.)
- [ ] Show visual feedback for edited cells
- [ ] Option to use keyboard shortcuts for quick editing
- [ ] Graceful fallback to side-bar detail when inline editing is not suitable

## Implementation Notes

- Related to the existing row detail editor (rowdetail.svelte)
- Should respect column types and validation rules
- Consider performance for large result sets
- May reuse input components from the row detail editor
