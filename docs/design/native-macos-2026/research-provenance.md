# Research and clean-room provenance

Date observed: 2026-08-12

## Operator-provided direction

- TablePro is the primary design reference and may be observed as an installed
  application if public materials leave a visual ambiguity.
- The desired result should closely adapt its overall native design language.

## TablePro public observations

Sources: [TablePro product page](https://tablepro.app/), [feature
overview](https://docs.tablepro.app/features/overview), [SQL editor
documentation](https://docs.tablepro.app/features/sql-editor), and [data grid
documentation](https://docs.tablepro.app/features/data-grid).

Observed commonplace/native patterns:

- edge-to-edge macOS window with a floating leading sidebar;
- compact toolbar clusters and a central connection/environment context;
- horizontal object/query tabs directly below the toolbar;
- opaque, dense, alternating-row data grid as the dominant surface;
- bottom mode/status/paging controls;
- optional trailing typed-value inspector;
- editor-above/results-below SQL workspace;
- compact filter rail above the grid.

Influence mapping:

| Observation | Adaptation |
|---|---|
| Sidebar + context toolbar + tabs + grid rhythm | Native Workbench structure |
| Editor/results split | Native Workbench SQL surface |
| Trailing inspector | Native Workbench typed value detail |
| Filter rail and bottom status/paging | Shared Data Grid components |
| Native density and restrained hierarchy | Shared typography/spacing direction |

No source code, tests, source comments, bundle internals, proprietary assets,
branding, product strings, stored credentials, or real data were inspected.
The installed application supplied visual comparison only. No implementation
material was inspected.

## Apple primary sources

- [Human Interface Guidelines: Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [Human Interface Guidelines: Designing for macOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-macos/)
- [Human Interface Guidelines: Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars)
- [Human Interface Guidelines: Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)
- [Human Interface Guidelines: Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility)
- [Applying Liquid Glass to custom views](https://developer.apple.com/documentation/SwiftUI/Applying-Liquid-Glass-to-custom-views)
- [AppKit in the new design system](https://developer.apple.com/videos/play/wwdc2025/310/)

The installed stable macOS 26.5 SDK is authoritative for implementation API
availability. No macOS 27 beta-only API is allowed.
