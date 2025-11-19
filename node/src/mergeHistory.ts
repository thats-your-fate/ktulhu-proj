// mergeHistory.ts
export interface Fact {
  entity: string;
  aspect?: string;
  value?: string;
  attributes?: Record<string, any>;
  [k: string]: any;
}

export interface ChatState {
  intents: string[];
  facts: Fact[];
  summary: string | null;
}

export interface HistoryEntry {
  state_delta?: {
    user_intent?: string;
    new_facts?: Fact[];
    message_summary?: string;
    [k: string]: any;
  };
  ts?: number;
}

export function mergeHistory(history: HistoryEntry[]): ChatState {
  const out: ChatState = {
    intents: [],
    facts: [],
    summary: null
  };

  for (const entry of history) {
    if (!entry?.state_delta) continue;
    const delta = entry.state_delta;

    // -- Intent merging --
    if (delta.user_intent && !out.intents.includes(delta.user_intent)) {
      out.intents.push(delta.user_intent);
    }

    // -- Facts merging --
    if (Array.isArray(delta.new_facts)) {
      for (const fact of delta.new_facts) {
        // avoid duplicate near-identical facts
        if (!out.facts.some(f => JSON.stringify(f) === JSON.stringify(fact))) {
          out.facts.push(fact);
        }
      }
    }

    // -- Summary merging --
    if (delta.message_summary) {
      out.summary = delta.message_summary;
    }
  }

  return out;
}
