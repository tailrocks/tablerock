# Phase 1 Bounded Ingress Evidence

## Checkpoint

TableRock replaces the raw root `mpsc` sender with one generic post-mapping
ingress contract. It carries semantic root messages, not database client values
or a placeholder engine-event envelope. Phase 2 remains the sole owner of
profile/session/context/query/result/operation identities and revisions.

The ingress has two type-distinct bounded delivery classes. The Phase 1 root
sender accepts semantic `Message` state events but uses an uninhabited
`RootProgress`, making accidental coalescing of any current state, completion,
failure, safety, or terminal fact unrepresentable:

- state transitions use the declared 256-slot queue and are never silently
  discarded; overflow collapses into one higher-priority `ResyncRequired` fact;
- progress occupies one latest-only slot, so any update rate has constant
  memory and cannot fill or precede the state-transition queue.

All publication, overflow, closure, and receive selection linearize through one
short critical section. The receiver therefore always observes pending resync
first, then accepted state transitions, then the latest progress, including
under concurrent publication. Stream closure drains accepted state and progress
before returning `None`. Progress publication and receiver closure share one
locked state transition, so closure either follows an accepted publication or
returns the undelivered value to the producer. The root adapter maps overflow
into `EngineResyncRequired`; the pure
reducer exposes a persistent non-color `Resync required` status until a mapped
`EngineResynchronized` snapshot fact clears it. The actual engine snapshot
request and revision validation begin with the Phase 2 service contract.

Shutdown signals remain the first biased branch in the executable loop. A
nested fair selection arbitrates ingress against terminal input, so continuous
producer traffic cannot starve terminal events, EOF, or errors.

## Evidence

- Public-seam tests prove hard-capacity acceptance followed by one explicit
  resync fact and ordered delivery of surviving transitions.
- Repeated progress publication proves latest-only coalescing and state-first
  delivery.
- A 10,000-progress/9,998-overflow stress fixture proves bounded collapse to
  one resync fact, one accepted state transition, and one latest progress fact.
- Closure tests prove drain-before-end and undelivered-value recovery.
- A 1,000-race concurrent close/publication fixture exercises the locked
  linearization boundary.
- A separate 1,000-race event/progress fixture proves event-first selection
  after simultaneous publication.
- A real PTY child continuously publishes root state while terminal Ctrl-C
  still reaches the production loop, exits successfully, and restores termios
  plus every writer-backed mode in order.
- Reducer/render tests prove idempotent resync state, visible projection, and
  explicit reconciliation.
- The transport is generic over distinct already-mapped event/progress types;
  no raw engine event can bypass the future Phase 2 identity/revision mapper.

External concept: bounded queue and latest-value coalescing only  
Public source: <https://docs.rs/tokio/1.52.3/tokio/sync/index.html>  
TableRock requirement: Roadmap Phase 1, TEA subscription contract, and quality
plan bounded-queue/resync rows  
Implementation source: TableRock requirements, Tokio public synchronization
APIs, and independent tests  
Copied code/assets/text: none
