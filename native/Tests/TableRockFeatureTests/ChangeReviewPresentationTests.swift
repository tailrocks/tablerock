import Foundation
import Testing

@testable import TableRockFeature

@Test func changeReviewLedgerChipOnlyWhenPending() {
  #expect(ChangeReviewPresentation.ledgerChip(entryCount: 0, reviewOpen: false) == nil)
  #expect(ChangeReviewPresentation.ledgerChip(entryCount: 3, reviewOpen: false) == "ledger 3")
  #expect(
    ChangeReviewPresentation.ledgerChip(entryCount: 1, reviewOpen: true)
      == "ledger 1 · review open")
}

@Test func changeReviewOutcomeFactIsNonColor() {
  let outcome = WorkbenchApplyOutcome(
    transaction: "committed", changeCount: 2, appliedCount: 2, conflictCount: 0, failedCount: 0)
  let fact = ChangeReviewPresentation.outcomeFact(outcome)
  #expect(fact.contains("committed"))
  #expect(fact.contains("2 applied"))
  #expect(fact.contains("2 planned"))
}

@Test func changeReviewExpiryAndMetadata() {
  #expect(ChangeReviewPresentation.expiryFact(expiresAtMs: 1_000, nowMs: 1_000) == "EXPIRED")
  #expect(ChangeReviewPresentation.expiryFact(expiresAtMs: 61_000, nowMs: 1_000) == "expires in 60s")
  let strip = ChangeReviewPresentation.metadataStrip(
    target: "public.users",
    expiresAtMs: 61_000,
    nowMs: 1_000,
    destructive: true,
    extra: "probe")
  #expect(strip.contains("public.users"))
  #expect(strip.contains("DESTRUCTIVE"))
  #expect(strip.contains("consume-once"))
  #expect(strip.contains("probe"))
}

@Test func changeReviewKindWordFromPreview() {
  #expect(
    ChangeReviewPresentation.kindWord(preview: "DELETE FROM t WHERE id = $1", destructive: true)
      == "DELETE")
  #expect(
    ChangeReviewPresentation.kindWord(preview: "ALTER TABLE t DROP COLUMN c", destructive: true)
      == "ALTER")
  #expect(ChangeReviewPresentation.kindWord(preview: "mystery", destructive: true) == "DESTRUCTIVE")
}

@Test func changeReviewProbeContractIsDocumented() {
  #expect(ChangeReviewPresentation.probeKindWord == "DELETE")
  #expect(ChangeReviewPresentation.probePreview.contains("DELETE"))
  #expect(ChangeReviewPresentation.probePreview.contains("$1"))
  #expect(ChangeReviewPresentation.probeDestructive)
  #expect(ChangeReviewPresentation.probeLedgerCount == 1)
}

@Test func workbenchStatusFactsIncludesLedgerChip() {
  let line = WorkbenchStatusFacts.line(
    operation: "READY",
    engine: "postgresql",
    querySummary: nil,
    queryError: nil,
    cancelOutcome: nil,
    catalogSummary: nil,
    catalogError: nil,
    resultRowCount: 10,
    production: true,
    ledgerEntryCount: 1,
    reviewOpen: true)
  #expect(line.contains("ledger 1 · review open"))
  #expect(line.contains("writes need review"))
}
