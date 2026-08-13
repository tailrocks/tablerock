import Foundation

@MainActor
extension WorkbenchPresentationStore {
  func showRedisOverview() async {
    guard connectedEngine == "redis", let client, let session = sessionData,
      !redisOverviewLoading
    else { return }
    redisOverviewPresented = true
    redisOverviewLoading = true
    redisOverviewError = nil
    defer { redisOverviewLoading = false }
    do {
      redisOverview = try await client.redisOverview(sessionId: session)
    } catch {
      redisOverview = nil
      redisOverviewError = "Redis overview failed: \(error)"
    }
  }

  func showRedisSubscription() {
    guard connectedEngine == "redis", sessionData != nil else { return }
    redisSubscriptionPresented = true
    redisSubscriptionError = nil
  }

  func startRedisSubscription() async {
    guard let client, let session = sessionData, !redisSubscriptionStarting,
      !redisSubscriptionIsActive
    else { return }
    let selector = redisSubscriptionSelector.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !selector.isEmpty else {
      redisSubscriptionError = "Enter a channel or pattern"
      return
    }
    redisSubscriptionStarting = true
    redisSubscriptionError = nil
    defer { redisSubscriptionStarting = false }
    do {
      let operation = try await client.startRedisSubscription(
        sessionId: session, selector: selector, pattern: redisSubscriptionPattern)
      redisSubscriptionStatus = try await client.redisSubscriptionStatus(operationId: operation)
      beginRedisSubscriptionPolling(operation)
    } catch {
      redisSubscriptionStatus = nil
      redisSubscriptionError = "Subscription failed: \(error)"
    }
  }

  func refreshRedisSubscription() async {
    guard let client, let operation = redisSubscriptionStatus?.operationId else { return }
    do {
      let status = try await client.redisSubscriptionStatus(operationId: operation)
      redisSubscriptionStatus = status
      if !redisSubscriptionIsActive { redisSubscriptionPollTask?.cancel() }
    } catch {
      redisSubscriptionError = "Subscription status unavailable: \(error)"
      redisSubscriptionPollTask?.cancel()
    }
  }

  private func beginRedisSubscriptionPolling(_ operation: Data) {
    redisSubscriptionPollTask?.cancel()
    redisSubscriptionPollTask = Task { [weak self] in
      while !Task.isCancelled {
        try? await Task.sleep(for: .milliseconds(250))
        guard !Task.isCancelled, let self,
          self.redisSubscriptionStatus?.operationId == operation,
          self.redisSubscriptionPresented
        else { return }
        await self.refreshRedisSubscription()
        if !self.redisSubscriptionIsActive { return }
      }
    }
  }

  func cancelRedisSubscription() async {
    guard let client, let operation = redisSubscriptionStatus?.operationId else { return }
    do {
      _ = try await client.cancelRedisSubscription(operationId: operation)
      await refreshRedisSubscription()
    } catch {
      redisSubscriptionError = "Cancel failed: \(error)"
    }
  }

  func closeRedisSubscription() async {
    if redisSubscriptionIsActive { await cancelRedisSubscription() }
    redisSubscriptionPollTask?.cancel()
    redisSubscriptionPollTask = nil
    redisSubscriptionPresented = false
  }
}
