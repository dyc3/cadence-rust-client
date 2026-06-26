# Cadence Rust Client

Domain vocabulary for the Cadence Rust client. The architecture follows the
Uber Cadence Go client (the source of truth per `AGENTS.md`); terms here favour
the names that client uses.

## Language

**DataConverter**:
The single seam for serializing workflow/activity inputs, outputs, and signals to
bytes and back. Configured once on both the client and the worker — Cadence carries
no per-payload encoding tag, so both sides must agree out of band. JSON is the
default adapter.
_Avoid_: serializer, codec, encoder, marshaller.

**Payload**:
The opaque byte form of a value once a `DataConverter` has encoded it. On the wire
it is `{ data: bytes }` with no format metadata — the decoder relies on its
configured `DataConverter`, not on the payload describing itself.
_Avoid_: blob, message, encoded value.

**Adapter** (of `DataConverter`):
A concrete converter satisfying the seam. `JsonDataConverter` is the production
adapter; a framing/fake converter proves the seam is real and swappable in tests.
Generic `encode`/`decode` live on the `DataConverterExt` extension trait so the
seam itself stays object-safe (`Arc<dyn DataConverter>`).
