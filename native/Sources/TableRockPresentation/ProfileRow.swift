import SwiftUI
import TableRockFeature

struct ProfileRow: View {
  let profile: WorkbenchProfileItem
  let connectionState: String
  var isActive: Bool = false

  private var targetSummary: String {
    [
      [
        [profile.host, profile.port].compactMap { $0 }.joined(separator: ":"),
        profile.context,
      ].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: "/"),
      profile.safetyMode == "read_only" ? "Read only" : "Confirm writes",
    ].compactMap { value in value?.isEmpty == false ? value : nil }.joined(separator: " · ")
  }

  private var live: (word: String, detail: String?) {
    ProfileLiveStatePresentation.parts(from: connectionState)
  }

  var body: some View {
    HStack(alignment: .top, spacing: 8) {
      Text(ProfileEngineBadge.code(profile.engine))
        .font(.caption.weight(.bold).monospaced())
        .frame(width: 28, alignment: .center)
        .padding(.vertical, 2)
        .background(
          .quaternary.opacity(0.5), in: RoundedRectangle(cornerRadius: 4, style: .continuous)
        )
        .accessibilityLabel(ProfileEngineBadge.accessibilityName(profile.engine))

      VStack(alignment: .leading, spacing: 2) {
        HStack(spacing: 6) {
          if profile.favorite {
            Image(systemName: "star.fill")
              .foregroundStyle(.yellow)
              .font(.caption2)
              .accessibilityLabel("Favorite")
          }
          Text(profile.name)
            .font(.body.weight(.medium))
            .lineLimit(1)
          if isActive {
            Text("ACTIVE")
              .font(.caption2.weight(.bold).monospaced())
              .accessibilityLabel("Active in this window")
          }
          if profile.productionWarning {
            Text("HALO PRODUCTION")
              .font(.caption2.weight(.bold))
              .accessibilityLabel("Production environment")
          } else if let environment = profile.environment, !environment.isEmpty {
            Text("HALO \(environment.uppercased())")
              .font(.caption2.weight(.semibold))
              .foregroundStyle(.secondary)
              .accessibilityLabel("Environment \(environment)")
          }
        }
        if !targetSummary.isEmpty {
          Text(targetSummary)
            .font(.caption)
            .foregroundStyle(.secondary)
            .lineLimit(1)
            .truncationMode(.middle)
        }
        HStack(spacing: 4) {
          Text(live.word)
            .font(.caption2.weight(.semibold).monospaced())
          if let detail = live.detail {
            Text(detail)
              .font(.caption2)
              .foregroundStyle(.secondary)
          }
          if profile.dangerousPlaintext {
            Text("PLAINTEXT SECRET")
              .font(.caption2.weight(.bold))
              .accessibilityLabel("Password stored as acknowledged plaintext")
          }
        }
        .foregroundStyle(.secondary)
      }
      Spacer(minLength: 0)
    }
    .padding(.vertical, 3)
    .accessibilityElement(children: .combine)
    .accessibilityLabel(accessibilitySummary)
  }

  private var accessibilitySummary: String {
    var parts = [
      ProfileEngineBadge.accessibilityName(profile.engine),
      profile.name,
      ProfileLiveStatePresentation.line(from: connectionState),
    ]
    if isActive { parts.append("active in this window") }
    if profile.productionWarning {
      parts.append("production environment")
    } else if let environment = profile.environment, !environment.isEmpty {
      parts.append("environment \(environment)")
    }
    if profile.dangerousPlaintext { parts.append("plaintext secret warning") }
    if !targetSummary.isEmpty { parts.append(targetSummary) }
    return parts.joined(separator: ", ")
  }
}
