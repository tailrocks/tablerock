import SwiftUI

struct LabConnectionSetupSheet: View {
    @EnvironmentObject private var session: LabSession

    @State private var name = "Northstar Analytics"
    @State private var host = "analytics.internal"
    @State private var port = "5432"
    @State private var database = "analytics"
    @State private var username = "table_operator"
    @State private var requireTLS = true
    @State private var safeMode = true
    @State private var testResult: String?

    var body: some View {
        NavigationStack {
            Form {
                Section("Identity") {
                    TextField("Name", text: $name)
                    Picker("Engine", selection: $session.engine) {
                        ForEach(LabEngine.allCases) { engine in
                            Text(engine.title).tag(engine)
                        }
                    }
                }

                Section("Server") {
                    TextField("Host", text: $host)
                    TextField("Port", text: $port)
                    TextField("Database", text: $database)
                    TextField("User", text: $username)
                    SecureField("Password", text: .constant("preview-only"))
                }

                Section("Safety") {
                    Toggle("Require TLS", isOn: $requireTLS)
                    Toggle("Review writes before execution", isOn: $safeMode)
                }

                if let testResult {
                    Section("Connection Test") {
                        Label(testResult, systemImage: "checkmark.circle.fill")
                            .foregroundStyle(.green)
                    }
                }
            }
            .formStyle(.grouped)
            .navigationTitle("New Connection")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { session.connectionSheetPresented = false }
                }
                ToolbarItemGroup(placement: .confirmationAction) {
                    Button("Test Connection") {
                        testResult = "Static preview validated"
                    }
                    .accessibilityIdentifier("design-lab-test-connection")
                    Button("Save & Open") {
                        session.connectionSheetPresented = false
                        session.show(.dataGrid)
                    }
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("design-lab-save-connection")
                }
            }
        }
        .frame(minWidth: 620, minHeight: 560)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("New connection setup")
        .accessibilityIdentifier("design-lab-connection-sheet")
    }
}

struct LabSafeEditSheet: View {
    @EnvironmentObject private var session: LabSession

    @State private var plan = ""
    @State private var seats = ""

    var body: some View {
        NavigationStack {
            Form {
                Section("Selected row") {
                    LabeledContent("Object", value: session.selectedCatalogItem.name)
                    LabeledContent("Row", value: String(session.selectedRow?.id ?? 0))
                    LabeledContent(
                        "Company",
                        value: session.selectedRow?.values[safe: 1] ?? "No selection"
                    )
                }

                Section("Proposed values") {
                    TextField("Plan", text: $plan)
                        .accessibilityIdentifier("design-lab-edit-plan")
                    TextField("Seats", text: $seats)
                        .accessibilityIdentifier("design-lab-edit-seats")
                }

                Section("Safety") {
                    Label(
                        "Changes are staged locally for review. Nothing executes from Design Lab.",
                        systemImage: "lock.shield"
                    )
                    .foregroundStyle(.secondary)
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Edit Selected Row")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { session.editSheetPresented = false }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Stage for Review") {
                        session.stageSafeEdit(plan: plan, seats: seats)
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(plan.isEmpty || seats.isEmpty)
                    .accessibilityIdentifier("design-lab-stage-edit")
                }
            }
        }
        .frame(minWidth: 560, minHeight: 430)
        .onAppear {
            plan = session.selectedRow?.values[safe: 3] ?? "Team"
            seats = session.selectedRow?.values[safe: 4] ?? "1"
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Safe row editor")
        .accessibilityIdentifier("design-lab-edit-sheet")
    }
}

struct LabQueryHistorySheet: View {
    @EnvironmentObject private var session: LabSession

    var body: some View {
        NavigationStack {
            List(session.queryHistory) { entry in
                Button {
                    session.openHistoryEntry(entry)
                } label: {
                    HStack(spacing: 12) {
                        Image(systemName: entry.engine.symbol)
                            .foregroundStyle(.secondary)
                            .frame(width: 24)
                        VStack(alignment: .leading, spacing: 3) {
                            Text(entry.title)
                                .font(.headline)
                            Text(entry.statement.replacingOccurrences(of: "\n", with: " "))
                                .font(.caption.monospaced())
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                        }
                        Spacer()
                        VStack(alignment: .trailing, spacing: 3) {
                            Text(entry.executedAt)
                            Text("\(entry.rowCount) · \(entry.duration)")
                                .foregroundStyle(.secondary)
                        }
                        .font(.caption)
                    }
                    .contentShape(.rect)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("design-lab-history-\(entry.id)")
            }
            .navigationTitle("Query History")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { session.historySheetPresented = false }
                }
            }
        }
        .frame(minWidth: 720, minHeight: 500)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Query history")
        .accessibilityIdentifier("design-lab-history-sheet")
    }
}

struct LabChangeReviewSheet: View {
    @EnvironmentObject private var session: LabSession
    @FocusState private var confirmationFocused: Bool

    @State private var confirmation = ""

    private var destructive: Bool {
        session.hasDestructiveChanges
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .top, spacing: 14) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.title)
                    .foregroundStyle(.red)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 4) {
                    Text(destructive ? "Review destructive changes" : "Review staged changes")
                        .font(.title2.weight(.semibold))
                    Text(
                        destructive
                            ? "\(session.pendingChangeCount) staged operations will run in one transaction. One operation deletes a row."
                            : "\(session.pendingChangeCount) safe update\(session.pendingChangeCount == 1 ? "" : "s") will run in one transaction."
                    )
                        .foregroundStyle(.secondary)
                }
                Spacer()
                LabBadge(
                    text: destructive ? "DESTRUCTIVE" : "SAFE CHANGE",
                    tint: destructive ? .red : .green,
                    symbol: destructive ? "trash.fill" : "lock.shield.fill"
                )
            }
            .padding(22)

            Divider()

            List(session.stagedChanges) { change in
                HStack(alignment: .top, spacing: 12) {
                    LabBadge(
                        text: change.kind.rawValue,
                        tint: change.kind == .delete ? .red : .orange
                    )
                    VStack(alignment: .leading, spacing: 3) {
                        Text(change.object).fontWeight(.semibold)
                        Text("\(change.field): \(change.before) → \(change.after)")
                            .font(.callout.monospaced())
                    }
                    Spacer()
                }
                .padding(.vertical, 4)
            }
            .listStyle(.inset)

            Divider()

            if destructive {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Type APPLY to enable the destructive action.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    TextField("APPLY", text: $confirmation)
                        .focused($confirmationFocused)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityIdentifier("design-lab-review-confirmation")
                }
                .padding(.horizontal, 22)
                .padding(.top, 14)
            }

            HStack {
                Button("Discard Changes", role: .destructive) {
                    session.discardChanges()
                }
                Spacer()
                Button("Back to Editing") { session.reviewSheetPresented = false }
                Button(destructive ? "Apply on PRODUCTION" : "Apply Changes", role: destructive ? .destructive : nil) {
                    session.applyChanges()
                }
                .disabled(destructive && confirmation != "APPLY")
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("design-lab-apply-changes")
            }
            .padding(22)
        }
        .frame(minWidth: 680, minHeight: 590)
        .interactiveDismissDisabled(destructive && confirmation.isEmpty == false)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(destructive ? "Destructive change review" : "Safe change review")
        .accessibilityIdentifier("design-lab-review-sheet")
        .task {
            guard destructive else { return }
            confirmationFocused = true
        }
    }
}

private extension Array {
    subscript(safe index: Index) -> Element? {
        indices.contains(index) ? self[index] : nil
    }
}
