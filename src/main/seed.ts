export const starterDiagrams = [
  {
    title: "Creative review",
    source: `flowchart LR
  brief["Creative brief"] --> concept["First concept"]
  concept --> review{"Review"}
  review -->|Revise| concept
  review -->|Approve| release["Ready to publish"]
  classDef highlight fill:#f2dfd1,stroke:#b96d49,color:#37291f
  class review highlight`,
  },
  {
    title: "Asset handoff",
    source: `sequenceDiagram
  actor Designer
  participant Library
  actor Producer
  Designer->>Library: Upload final assets
  Producer->>Library: Request approved assets
  Library-->>Producer: Return the approved version`,
  },
  {
    title: "Production states",
    source: `stateDiagram-v2
  [*] --> Draft
  Draft --> InReview
  InReview --> Draft: Changes requested
  InReview --> Approved
  Approved --> Published
  Published --> [*]`,
  },
]
