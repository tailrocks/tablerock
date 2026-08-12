import SwiftUI

struct LabConnectionSetupSheet: View {
    @Environment(\.dismiss) private var dismiss
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
                    Button("Cancel") { dismiss() }
                }
                ToolbarItemGroup(placement: .confirmationAction) {
                    Button("Test Connection") {
                        testResult = "Static preview validated"
                    }
                    .accessibilityIdentifier("design-lab-test-connection")
                    Button("Save & Open") {
                        dismiss()
                        session.show(.dataGrid)
                    }
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("design-lab-save-connection")
                }
            }
        }
        .frame(minWidth: 620, minHeight: 560)
        .accessibilityIdentifier("design-lab-connection-sheet")
    }
}

struct LabDestructiveReviewSheet: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject private var session: LabSession

    @State private var confirmation = ""

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .top, spacing: 14) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.title)
                    .foregroundStyle(.red)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 4) {
                    Text("Review changes on PRODUCTION")
                        .font(.title2.weight(.semibold))
                    Text("Four staged operations will run in one transaction. One operation deletes a row.")
                        .foregroundStyle(.secondary)
                }
                Spacer()
                LabBadge(text: "DESTRUCTIVE", tint: .red, symbol: "trash.fill")
            }
            .padding(22)

            Divider()

            List(LabFixtures.changes) { change in
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

            VStack(alignment: .leading, spacing: 8) {
                Text("Type APPLY to enable the destructive action.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                TextField("APPLY", text: $confirmation)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityIdentifier("design-lab-review-confirmation")
            }
            .padding(.horizontal, 22)
            .padding(.top, 14)

            HStack {
                Button("Discard Changes", role: .destructive) {
                    dismiss()
                    session.show(.dataGrid)
                }
                Spacer()
                Button("Back to Editing") { dismiss() }
                Button("Apply on PRODUCTION", role: .destructive) {
                    dismiss()
                    session.show(.dataGrid)
                }
                .disabled(confirmation != "APPLY")
                .keyboardShortcut(.defaultAction)
                .accessibilityIdentifier("design-lab-apply-production")
            }
            .padding(22)
        }
        .frame(minWidth: 680, minHeight: 590)
        .interactiveDismissDisabled(confirmation.isEmpty == false)
        .accessibilityIdentifier("design-lab-review-sheet")
    }
}
