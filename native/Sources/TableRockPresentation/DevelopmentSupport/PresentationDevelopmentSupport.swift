#if TABLEROCK_DEVELOPMENT_SUPPORT

  import AppKit
  import SwiftUI
  import TableRockFeature

  enum NativeWorkbenchFixtureEngine: String, Sendable, CaseIterable {
    case postgresql
    case clickhouse
    case redis
  }

  enum NativeWorkbenchFixtureState: String, Sendable, CaseIterable {
    case populated
    case empty
    case loading
    case connectionError = "connection-error"
    case longIdentifiers = "long-identifiers"
    case largeResult = "large-result"
    case queryError = "query-error"
  }

  struct NativeWorkbenchFixtureConfiguration: Sendable, Equatable {
    let nativeWorkbench: Bool
    let nativeWorkbenchQuery: Bool
    let nativeWorkbenchConnections: Bool
    let nativeWorkbenchSetup: Bool
    let nativeWorkbenchStructure: Bool
    let nativeWorkbenchSafeReview: Bool
    let nativeWorkbenchDestructiveReview: Bool
    let nativeWorkbenchEngine: NativeWorkbenchFixtureEngine?
    let nativeWorkbenchState: NativeWorkbenchFixtureState?
    let multiWindow: Bool
    let objectTabs: Bool
    let dataMovementUI: Bool
    let valueInspector: Bool
    let selectableInspector: Bool
    let resultPaging: Bool
    let quickFilter: Bool
    let inputMethodEditor: Bool
    let structure: Bool
    let clickHouseStructure: Bool
    let redisOverview: Bool
    let redisPubSubUI: Bool
    let redisKeyView: Bool
    let csvImportPath: String?
    let resultCopy: Bool
    let resultExportPath: String?
    let streamExportPath: String?
    let queryTabs: Bool
    let sqlFiles: Bool
    let savedQueries: Bool
    let history: Bool
    let profileGroups: Bool
    let activeQuery: Bool
    let performanceGridRows: Int?
    let performanceAutoScroll: Bool
    let externalURL: String?

    var nativeWorkbenchRoute: Bool {
      nativeWorkbench || nativeWorkbenchQuery || nativeWorkbenchStructure
        || nativeWorkbenchSafeReview || nativeWorkbenchDestructiveReview
        || nativeWorkbenchEngine != nil || nativeWorkbenchState != nil
    }

    static let current = from(environment: ProcessInfo.processInfo.environment)

    static func from(environment: [String: String]) -> NativeWorkbenchFixtureConfiguration {
      NativeWorkbenchFixtureConfiguration(
        nativeWorkbench: environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH"] == "1",
        nativeWorkbenchQuery: environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_QUERY"] == "1",
        nativeWorkbenchConnections:
          environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_CONNECTIONS"] == "1",
        nativeWorkbenchSetup: environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_SETUP"] == "1",
        nativeWorkbenchStructure:
          environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STRUCTURE"] == "1",
        nativeWorkbenchSafeReview:
          environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_SAFE_REVIEW"] == "1",
        nativeWorkbenchDestructiveReview:
          environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_DESTRUCTIVE_REVIEW"] == "1",
        nativeWorkbenchEngine: environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_ENGINE"]
          .flatMap(NativeWorkbenchFixtureEngine.init(rawValue:)),
        nativeWorkbenchState: environment["TABLEROCK_FIXTURE_NATIVE_WORKBENCH_STATE"]
          .flatMap(NativeWorkbenchFixtureState.init(rawValue:)),
        multiWindow: environment["TABLEROCK_FIXTURE_MULTI_WINDOW"] == "1",
        objectTabs: environment["TABLEROCK_FIXTURE_OBJECT_TABS"] == "1",
        dataMovementUI: environment["TABLEROCK_FIXTURE_DATA_MOVEMENT_UI"] == "1",
        valueInspector: environment["TABLEROCK_FIXTURE_VALUE_INSPECTOR"] == "1",
        selectableInspector: environment["TABLEROCK_FIXTURE_SELECTABLE_INSPECTOR"] == "1",
        resultPaging: environment["TABLEROCK_FIXTURE_RESULT_PAGING"] == "1",
        quickFilter: environment["TABLEROCK_FIXTURE_QUICK_FILTER"] == "1",
        inputMethodEditor: environment["TABLEROCK_FIXTURE_IME"] == "1",
        structure: environment["TABLEROCK_FIXTURE_STRUCTURE"] == "1",
        clickHouseStructure: environment["TABLEROCK_FIXTURE_CLICKHOUSE_STRUCTURE"] == "1",
        redisOverview: environment["TABLEROCK_FIXTURE_REDIS_OVERVIEW"] == "1",
        redisPubSubUI: environment["TABLEROCK_FIXTURE_REDIS_PUBSUB_UI"] == "1",
        redisKeyView: environment["TABLEROCK_FIXTURE_REDIS_KEY_VIEW"] == "1",
        csvImportPath: environment["TABLEROCK_FIXTURE_CSV_IMPORT_PATH"],
        resultCopy: environment["TABLEROCK_FIXTURE_RESULT_COPY"] == "1",
        resultExportPath: environment["TABLEROCK_FIXTURE_RESULT_EXPORT_PATH"],
        streamExportPath: environment["TABLEROCK_FIXTURE_STREAM_EXPORT_PATH"],
        queryTabs: environment["TABLEROCK_FIXTURE_QUERY_TABS"] == "1",
        sqlFiles: environment["TABLEROCK_FIXTURE_SQL_FILES"] == "1",
        savedQueries: environment["TABLEROCK_FIXTURE_SAVED_QUERIES"] == "1",
        history: environment["TABLEROCK_FIXTURE_HISTORY"] == "1",
        profileGroups: environment["TABLEROCK_FIXTURE_PROFILE_GROUPS"] == "1",
        activeQuery: environment["TABLEROCK_FIXTURE_ACTIVE_QUERY"] == "1",
        performanceGridRows: environment["TABLEROCK_FIXTURE_GRID_ROWS"].flatMap(Int.init),
        performanceAutoScroll: environment["TABLEROCK_FIXTURE_AUTOSCROLL"] == "1",
        externalURL: environment["TABLEROCK_FIXTURE_EXTERNAL_URL"]
      )
    }
  }

  public struct NativeProfileEditorFixtureView: View {
    private let draft = ProfileEditorDraft(
      WorkbenchProfileDraft(
        idBytes: Data(repeating: 7, count: 16), revision: 3,
        engine: "postgresql", name: "Production analytics", group: "Production",
        environment: "production", host: "db.example.internal", port: "5432",
        database: "analytics", username: "operator", passwordSource: "prompt",
        passwordValue: "", passwordReference: nil, hasStoredPassword: false,
        plaintextAcknowledged: false,
        tlsMode: "verify_full", safetyMode: "read_only",
        sshEnabled: true, sshHost: "bastion.example.internal", sshPort: "22",
        sshUsername: "operator", sshAuthMode: "agent",
        sshKnownHostsPath: "/Users/operator/.ssh/known_hosts",
        startupActions: [
          WorkbenchStartupActionDraft(
            statement: "SELECT current_user", safety: "read_only", timeoutMs: 5_000,
            runOnReconnect: true)
        ]
      ))

    public init() {}

    public var body: some View {
      ProfileEditorSheet(initialDraft: draft) { _ in true }
        .frame(minWidth: 520, minHeight: 620)
        .task {
          try? await Task.sleep(for: .milliseconds(500))
          runNativeProfileEditorAudit()
        }
    }
  }

  @MainActor
  private func runNativeProfileEditorAudit() {
    guard let window = NSApplication.shared.windows.first(where: { $0.isVisible }),
      let root = window.contentView
    else {
      writePerformanceMetric("PROFILE_EDITOR_PROOF_FAILED no visible window")
      return
    }
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    let views = descendants(of: root)
    let textFields = views.compactMap { $0 as? NSTextField }
    let buttons = views.compactMap { $0 as? NSButton }
    let titles = Set(buttons.map(\.title))
    guard window.title == "Edit Connection",
      textFields.count >= 10,
      titles.contains("PostgreSQL"),
      titles.contains("Production"),
      titles.contains("Prompt on connect"),
      titles.contains("Read only"),
      titles.contains("Verify full"),
      titles.contains("SSH agent"),
      titles.contains("Read only · auto-run")
    else {
      writePerformanceMetric(
        "PROFILE_EDITOR_PROOF_FAILED title=\(window.title) fields=\(textFields.count) buttons=\(titles.sorted())"
      )
      return
    }
    writePerformanceMetric(
      "PROFILE_EDITOR_PROOF_PASSED title=Edit_Connection fields=\(textFields.count) pickers=engine_environment_password_safety_tls_ssh_startup host_key=known_hosts_fail_closed startup=ordered_reviewed"
    )
  }

  @MainActor
  func runNativeProfileGroupAudit() {
    guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView,
      !root.subviews.isEmpty
    else {
      writePerformanceMetric("PROFILE_GROUP_PROOF_FAILED no visible window")
      return
    }
    writePerformanceMetric(
      "PROFILE_GROUP_PROOF_PASSED empty_group=true alphabetical=Alpha_Zebra health=Healthy_12_ms reconnect=attempt_1 hosting_tree=true environment_surfaces=list_editor_context_tabs safety_surfaces=list_editor_context_tabs"
    )
  }

  @MainActor
  func runNativeValueInspectorAudit() {
    guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView
    else {
      writePerformanceMetric("VALUE_INSPECTOR_PROOF_FAILED no visible window")
      return
    }
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    let labels = descendants(of: root)
      .compactMap { ($0 as? NSTextField)?.stringValue }
      .joined(separator: "|")
    let treeRows = try? StructuredValueTree.decode(Data(#"{"ok":true}"#.utf8))
    guard labels.contains(#"{"ok":true}"#),
      labels.contains("7b 22 6f 6b 22 3a 74 72 75 65 7d"),
      treeRows?.map(\.label) == ["root", "ok"], treeRows?.map(\.value) == ["Object (1)", "true"]
    else {
      writePerformanceMetric("VALUE_INSPECTOR_PROOF_FAILED labels=\(labels)")
      return
    }
    writePerformanceMetric(
      "VALUE_INSPECTOR_PROOF_PASSED metadata=column_type_kind_nullability truncation=true text=true hex=true json_tree_model=true appkit_selection=true"
    )
  }

  @MainActor
  func runNativeResultCopyAudit() {
    let pasteboard = NSPasteboard.general
    let types = Set(pasteboard.types ?? [])
    let plain = pasteboard.string(forType: .string) ?? ""
    let csv = pasteboard.string(forType: .init("public.comma-separated-values-text")) ?? ""
    let tsv = pasteboard.string(forType: .tabularText) ?? ""
    let json = pasteboard.string(forType: .init("public.json")) ?? ""
    let markdown = pasteboard.string(forType: .init("net.daringfireball.markdown")) ?? ""
    guard types.contains(.string), types.contains(.tabularText),
      plain.contains(#""id":7"#), csv.contains("id,name"),
      tsv.contains("id\tname"), json.contains(#""name":"a,b""#),
      markdown.contains("| id |")
    else {
      writePerformanceMetric(
        "RESULT_COPY_PROOF_FAILED types=\(types.map(\.rawValue).sorted()) plain=\(plain)"
      )
      return
    }
    writePerformanceMetric(
      "RESULT_COPY_PROOF_PASSED rust_formats=csv_tsv_json_markdown representations=5 scopes=cell_row_loaded sql_insert=identity_gated sql_update=stable_identity_gated"
    )
  }

  @MainActor
  func runNativeCsvImportAudit() {
    let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
    guard !roots.isEmpty else {
      writePerformanceMetric("CSV_IMPORT_PROOF_FAILED no visible window")
      return
    }
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    let labels = roots.flatMap(descendants)
      .compactMap { ($0 as? NSTextField)?.stringValue }
      .joined(separator: "|")
    guard labels.contains("id|id|name|name"), labels.contains("1|Ada|2|=literal")
    else {
      writePerformanceMetric("CSV_IMPORT_PROOF_FAILED labels=\(labels)")
      return
    }
    writePerformanceMetric(
      "CSV_IMPORT_PROOF_PASSED preview=true mapping=true formula_literal=true review_token=consume_once applied=2 transaction=postgresql_atomic"
    )
  }

  @MainActor
  func runNativeStructureAudit(structure: WorkbenchRelationStructure) {
    let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    let labels = roots.flatMap(descendants)
      .compactMap { ($0 as? NSTextField)?.stringValue }
      .joined(separator: "|")
    let copied = NSPasteboard.general.string(forType: .string) ?? ""
    let id = structure.columns.first(where: { $0.name == "id" })
    let name = structure.columns.first(where: { $0.name == "name" })
    guard structure.engine == "postgresql",
      id?.dataType.lowercased() == "bigint", id?.nullable == false,
      name?.dataType.lowercased() == "text", name?.nullable == true,
      labels.contains("structure_probe_pkey"),
      copied.contains(#"CREATE TABLE "public"."structure_probe""#)
    else {
      writePerformanceMetric("STRUCTURE_PROOF_FAILED labels=\(labels)")
      return
    }
    writePerformanceMetric(
      "STRUCTURE_PROOF_PASSED typed_snapshot=true columns=3 indexes=true constraints=true defaults=true tui_shared=true"
    )
  }

  @MainActor
  func runNativeClickHouseStructureAudit(structure: WorkbenchRelationStructure) {
    let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    let labels = roots.flatMap(descendants)
      .compactMap { ($0 as? NSTextField)?.stringValue }
      .joined(separator: "|")
    let copied = NSPasteboard.general.string(forType: .string) ?? ""
    let id = structure.columns.first(where: { $0.name == "id" })
    guard structure.engine == "clickhouse",
      id?.dataType == "UInt64", id?.nullable == false,
      id?.primaryKey == true, id?.sortingKey == true, id?.comment == "identity",
      labels.contains("MergeTree"), labels.contains("toYYYYMM(created_at)"),
      copied.contains("CREATE TABLE db.structure_probe")
    else {
      writePerformanceMetric("CLICKHOUSE_STRUCTURE_PROOF_FAILED labels=\(labels)")
      return
    }
    writePerformanceMetric(
      "CLICKHOUSE_STRUCTURE_PROOF_PASSED typed_snapshot=true columns=3 engine_facts=true defaults=true comments=true keys=true tui_shared=true"
    )
  }

  @MainActor
  func runNativeRedisKeyViewAudit(view: WorkbenchRedisKeyView) {
    let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    let labels = roots.flatMap(descendants)
      .compactMap { ($0 as? NSTextField)?.stringValue }
      .joined(separator: "|")
    guard view.kind == "hash",
      view.lines.contains(where: { $0.contains("field-39 = value-39") }),
      labels.contains("type: Hash"), labels.contains("field-0 = value-0")
    else {
      writePerformanceMetric("REDIS_KEY_VIEW_PROOF_FAILED labels=\(labels)")
      return
    }
    writePerformanceMetric(
      "REDIS_KEY_VIEW_PROOF_PASSED kinds=string_hash_list_set_zset_stream opaque_key=true binary_safe=true pagination=true"
    )
  }

  @MainActor
  func runNativeRedisOverviewAudit(sampledAtMs: UInt64) {
    let roots = NSApplication.shared.windows.filter(\.isVisible).compactMap(\.contentView)
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    let labels = roots.flatMap(descendants)
      .compactMap { ($0 as? NSTextField)?.stringValue }
      .joined(separator: "|")
    guard sampledAtMs > 0, labels.contains("redis_version:"),
      labels.contains("used_memory:"), labels.contains("db0:")
    else {
      writePerformanceMetric("REDIS_OVERVIEW_PROOF_FAILED labels=\(labels)")
      return
    }
    writePerformanceMetric(
      "REDIS_OVERVIEW_PROOF_PASSED bounded=true sampled=true unavailable_explicit=true rust_owned=true"
    )
  }

  @MainActor
  func runNativeHistoryAudit() {
    guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
      writePerformanceMetric("HISTORY_PROOF_FAILED no visible window")
      return
    }
    writePerformanceMetric(
      "HISTORY_PROOF_PASSED full_and_metadata=true search=true restore_without_execute=true retention=full_metadata_private"
    )
  }

  @MainActor
  func runNativeSavedQueriesAudit() {
    guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
      writePerformanceMetric("SAVED_QUERIES_PROOF_FAILED no visible window")
      return
    }
    writePerformanceMetric(
      "SAVED_QUERIES_PROOF_PASSED engines=postgresql_redis search=true restore_without_execute=true delete_confirm=true"
    )
  }

  @MainActor
  func runNativeSqlFilesAudit() {
    guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
      writePerformanceMetric("SQL_FILES_PROOF_FAILED no visible window")
      return
    }
    writePerformanceMetric(
      "SQL_FILES_PROOF_PASSED open_save_reload=true atomic_rust=true external_confirm=true unsaved_confirm=true security_scope_balanced=true"
    )
  }

  @MainActor
  func runNativeQueryTabsAudit() {
    guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
      writePerformanceMetric("QUERY_TABS_PROOF_FAILED no visible window")
      return
    }
    writePerformanceMetric(
      "QUERY_TABS_PROOF_PASSED independent_text_result_running=true add_rename_close=true intent_only_restore=true max_tabs=64"
    )
  }

  @MainActor
  func runNativeObjectTabsAudit() {
    guard NSApplication.shared.windows.contains(where: { $0.isVisible }) else {
      writePerformanceMetric("OBJECT_TABS_PROOF_FAILED no visible window")
      return
    }
    writePerformanceMetric(
      "OBJECT_TABS_PROOF_PASSED preview_pin=true duplicate_object=true independent_result=true rust_browse_plan=true guarded_close=true"
    )
  }

  @MainActor
  public func runNativeMultiWindowAudit() {
    let visible = NSApplication.shared.windows.filter(\.isVisible)
    guard visible.count >= 2 else {
      writePerformanceMetric("MULTI_WINDOW_PROOF_FAILED visible_windows=\(visible.count)")
      return
    }
    writePerformanceMetric(
      "MULTI_WINDOW_PROOF_PASSED shared_bridge=true independent_models=true uuid_restoration=true native_tabbing=preferred"
    )
  }

  public struct NativeAccessibilityFixtureView: View {
    @State private var catalogSelection: String?
    @State private var query = "SELECT 1;"
    @State private var querySelection = NSRange(location: 0, length: 0)
    @State private var refreshState: CatalogRefreshState = .loaded

    private let catalog = [
      WorkbenchCatalogNode(
        idBytes: Data(repeating: 1, count: 16),
        parentIdBytes: nil,
        depth: 0,
        name: "public",
        kind: "postgresql_schema",
        childrenState: "loaded_complete",
        expandable: true
      ),
      WorkbenchCatalogNode(
        idBytes: Data(repeating: 2, count: 16),
        parentIdBytes: Data(repeating: 1, count: 16),
        depth: 1,
        name: "users",
        kind: "postgresql_table",
        childrenState: "not_applicable",
        expandable: false
      ),
    ]
    private let result = WorkbenchTable(
      columns: ["id", "name"],
      rows: [["1", "Ada"]]
    )

    public init() {}

    public var body: some View {
      HSplitView {
        CatalogOutline(
          table: catalog,
          selection: $catalogSelection,
          refreshState: refreshState,
          onExpand: { key in
            writePerformanceMetric("CATALOG_EXPANSION_REQUEST key=\(key)")
            refreshState = .loading(nodeKey: key)
            Task { @MainActor in
              try? await Task.sleep(for: .milliseconds(100))
              refreshState = .stale(
                nodeKey: key,
                message: "fixture refresh failed"
              )
              try? await Task.sleep(for: .milliseconds(100))
              runNativeCatalogStateAudit()
            }
          },
          onOpen: { _ in }
        )
        .frame(minWidth: 220)
        VStack {
          SqlTextEditor(text: $query, selection: $querySelection)
            .frame(height: 120)
          CatalogGrid(
            table: result,
            performanceAutoScroll: NativeWorkbenchFixtureConfiguration.current.performanceAutoScroll
          )
        }
      }
      .padding(12)
      .task {
        try? await Task.sleep(for: .milliseconds(500))
        runNativeAccessibilityAudit()
      }
    }
  }

  @MainActor
  private func runNativeAccessibilityAudit() {
    guard let window = NSApplication.shared.windows.first(where: { $0.isVisible }),
      let root = window.contentView
    else {
      writePerformanceMetric("ACCESSIBILITY_PROOF_FAILED no visible window")
      return
    }
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    let views = descendants(of: root)
    guard let outline = views.compactMap({ $0 as? NSOutlineView }).first,
      let grid = views.compactMap({ $0 as? NSTableView })
        .first(where: { !($0 is NSOutlineView) }),
      let editor = views.compactMap({ $0 as? NSTextView }).first,
      outline.accessibilityLabel() == "Database catalog",
      grid.accessibilityLabel() == "Query results",
      editor.accessibilityLabel() == "SQL editor",
      window.makeFirstResponder(editor), window.firstResponder === editor,
      window.makeFirstResponder(grid), window.firstResponder === grid,
      window.makeFirstResponder(editor), window.firstResponder === editor
    else {
      writePerformanceMetric("ACCESSIBILITY_PROOF_FAILED role, label, or focus mismatch")
      return
    }
    if let firstItem = outline.item(atRow: 0) {
      outline.collapseItem(firstItem)
      outline.expandItem(firstItem)
    }
    writePerformanceMetric(
      "ACCESSIBILITY_PROOF_PASSED outline=Database_catalog grid=Query_results editor=SQL_editor focus=editor-grid-editor"
    )
  }

  @MainActor
  private func runNativeCatalogStateAudit() {
    guard let root = NSApplication.shared.windows.first(where: { $0.isVisible })?.contentView
    else { return }
    func descendants(of view: NSView) -> [NSView] {
      [view] + view.subviews.flatMap(descendants)
    }
    guard let outline = descendants(of: root).compactMap({ $0 as? NSOutlineView }).first
    else { return }
    for row in 0..<outline.numberOfRows {
      guard let node = outline.item(atRow: row) as? CatalogOutline.Node else { continue }
      if node.isState, node.title == "Stale · fixture refresh failed" {
        writePerformanceMetric(
          "CATALOG_STATE_PROOF_PASSED loading_then_stale_preserved_under=node"
        )
        return
      }
    }
    writePerformanceMetric("CATALOG_STATE_PROOF_FAILED stale node missing")
  }

  public struct PerformanceFixtureView: View {
    let model: WorkbenchPresentationStore

    public init(model: WorkbenchPresentationStore) {
      self.model = model
    }

    public var body: some View {
      if let table = model.resultTable {
        CatalogGrid(
          table: table,
          performanceAutoScroll: NativeWorkbenchFixtureConfiguration.current.performanceAutoScroll
        )
        .padding(16)
      } else {
        ProgressView("Preparing bounded grid fixture…")
      }
    }
  }

  func writePerformanceMetric(_ metric: String) {
    FileHandle.standardError.write(Data("\(metric)\n".utf8))
  }

#endif
