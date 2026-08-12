import Foundation
import SwiftUI

struct RedisOverviewSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Redis Overview", systemImage: "gauge.with.dots.needle.67percent")
          .font(.title2.bold())
        Spacer()
        Button("Refresh") { Task { await model.showRedisOverview() } }
          .disabled(model.redisOverviewLoading)
        Button("Close") { model.redisOverviewPresented = false }
      }
      if model.redisOverviewLoading {
        ProgressView("Loading bounded INFO snapshot…")
      }
      if let overview = model.redisOverview {
        Text("Sampled at \(overview.sampledAtMs) ms since Unix epoch")
          .font(.callout)
          .foregroundStyle(.secondary)
        ScrollView {
          LazyVStack(alignment: .leading, spacing: 5) {
            ForEach(overview.lines.indices, id: \.self) { index in
              Text(overview.lines[index])
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
            }
          }
          .frame(maxWidth: .infinity, alignment: .leading)
          .padding(8)
        }
      } else if !model.redisOverviewLoading && model.redisOverviewError == nil {
        ContentUnavailableView(
          "No Redis snapshot", systemImage: "gauge",
          description: Text("Refresh to sample current server facts.")
        )
      }
      if let error = model.redisOverviewError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
      }
    }
    .padding(20)
    .frame(minWidth: 680, minHeight: 520)
  }
}

struct RedisSubscriptionSheet: View {
  @Environment(WorkbenchPresentationStore.self) private var model

  var body: some View {
    @Bindable var model = model
    VStack(alignment: .leading, spacing: 14) {
      HStack {
        Label("Redis Pub/Sub", systemImage: "dot.radiowaves.left.and.right")
          .font(.title2.bold())
        Spacer()
        Button("Refresh") { Task { await model.refreshRedisSubscription() } }
          .disabled(model.redisSubscriptionStatus == nil)
        Button("Close") { Task { await model.closeRedisSubscription() } }
      }
      HStack(spacing: 10) {
        Picker("Mode", selection: $model.redisSubscriptionPattern) {
          Text("Channel").tag(false)
          Text("Pattern").tag(true)
        }
        .pickerStyle(.segmented)
        .frame(width: 190)
        .disabled(model.redisSubscriptionIsActive)
        TextField(
          model.redisSubscriptionPattern ? "Pattern" : "Channel",
          text: $model.redisSubscriptionSelector
        )
        .textFieldStyle(.roundedBorder)
        .disabled(model.redisSubscriptionIsActive)
        .accessibilityIdentifier("redis.pubsub.selector")
        Button("Subscribe") { Task { await model.startRedisSubscription() } }
          .buttonStyle(.glassProminent)
          .disabled(
            model.redisSubscriptionStarting || model.redisSubscriptionIsActive
              || model.redisSubscriptionSelector.trimmingCharacters(in: .whitespacesAndNewlines)
                .isEmpty
          )
          .accessibilityIdentifier("redis.pubsub.subscribe")
        Button("Cancel", role: .cancel) { Task { await model.cancelRedisSubscription() } }
          .disabled(!model.redisSubscriptionIsActive)
          .accessibilityIdentifier("redis.pubsub.cancel")
      }
      if model.redisSubscriptionStarting {
        ProgressView("Starting subscription…")
      }
      if let status = model.redisSubscriptionStatus {
        HStack(spacing: 12) {
          Text(status.pattern ? "PSUBSCRIBE" : "SUBSCRIBE").bold()
          Text(status.selector).font(.system(.body, design: .monospaced))
          Spacer()
          Text(status.phase.replacingOccurrences(of: "_", with: " ").capitalized)
          Text("\(status.totalReceived) received")
        }
        .foregroundStyle(.secondary)
        if status.discontinuities > 0 {
          Label(
            "\(status.discontinuities) delivery gap(s); displayed messages are not complete",
            systemImage: "exclamationmark.triangle.fill"
          )
          .foregroundStyle(.orange)
          .accessibilityIdentifier("redis.pubsub.gap")
        }
        GroupBox("Messages · newest retained window") {
          if status.messages.isEmpty {
            ContentUnavailableView(
              "Waiting for messages", systemImage: "ellipsis.message",
              description: Text("Published messages appear here until cancellation."))
          } else {
            ScrollView {
              LazyVStack(alignment: .leading, spacing: 6) {
                ForEach(Array(status.messages.enumerated()), id: \.offset) { _, message in
                  Text(message)
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
              }
              .padding(8)
            }
          }
        }
        Text(status.summary).font(.callout).foregroundStyle(.secondary)
          .accessibilityIdentifier("redis.pubsub.status")
          .accessibilityValue(status.summary)
      } else if !model.redisSubscriptionStarting && model.redisSubscriptionError == nil {
        ContentUnavailableView(
          "No active subscription", systemImage: "dot.radiowaves.left.and.right",
          description: Text("Choose a channel or pattern, then subscribe."))
      }
      if let error = model.redisSubscriptionError {
        Text(error).foregroundStyle(.red).textSelection(.enabled)
      }
    }
    .padding(20)
    .frame(minWidth: 760, minHeight: 560)
    .accessibilityElement(children: .contain)
    .interactiveDismissDisabled(model.redisSubscriptionIsActive)
  }
}
