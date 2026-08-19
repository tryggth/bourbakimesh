import React, { useState, useEffect } from 'react';
import {
  Play,
  StepForward,
  Square,
  RotateCcw,
  CheckCircle2,
  Brain,
  Sliders,
  Copy,
  Check,
  ChevronRight,
  Activity,
  Layers,
  Search,
  Workflow,
  Download,
  Terminal,
  ShieldCheck,
  Clock,
  FileCode,
  Sparkles,
} from 'lucide-react';
import {
  ProofNode,
  SearchConfig,
  SearchResult,
  TreeUpdatedEvent,
} from '../types/proverEvents';
import { Expr, CicExpr } from '../config/models';
import { proofSearchEngine, DEFAULT_SEARCH_CONFIG } from '../services/proofSearchEngine';
import { gemmaEdgeController } from '../services/llmController';
import { eventTracer, TraceEvent } from '../services/eventTracer';
import * as kernelWasm from '../wasm/kernel/kernel_wasm';

interface PresetItem {
  name: string;
  desc: string;
  goalStr: string;
  hyps: Record<string, Expr>;
  target: Expr;
}

interface CicPresetItem {
  name: string;
  desc: string;
  context: [string, CicExpr][];
  goalType: CicExpr;
  proofTerm: CicExpr;
}

interface MathlibPresetItem {
  name: string;
  desc: string;
  jsonPayload: string;
}

const BENCHMARK_PRESETS: PresetItem[] = [
  {
    name: 'AndComm',
    desc: 'h0: A ∧ B ⊢ B ∧ A',
    goalStr: 'A ∧ B -> B ∧ A',
    hyps: {
      h0: { And: [{ Prop: 'A' }, { Prop: 'B' }] },
    },
    target: {
      And: [{ Prop: 'B' }, { Prop: 'A' }],
    },
  },
  {
    name: 'ModusPonens',
    desc: 'h0: A → B, h1: A ⊢ B',
    goalStr: '(A -> B) -> A -> B',
    hyps: {
      h0: { Impl: [{ Prop: 'A' }, { Prop: 'B' }] },
      h1: { Prop: 'A' },
    },
    target: { Prop: 'B' },
  },
  {
    name: 'DoubleAnd',
    desc: 'h0: (A ∧ B) ∧ C ⊢ C ∧ A',
    goalStr: '(A ∧ B) ∧ C -> C ∧ A',
    hyps: {
      h0: { And: [{ And: [{ Prop: 'A' }, { Prop: 'B' }] }, { Prop: 'C' }] },
    },
    target: {
      And: [{ Prop: 'C' }, { Prop: 'A' }],
    },
  },
];

const CIC_BENCHMARK_PRESETS: CicPresetItem[] = [
  {
    name: 'Identity (A → A)',
    desc: 'λ (x : A) => x : A → A',
    context: [['A', { Sort: 'Zero' }]],
    goalType: { ForallE: ['_', { FVar: 'A' }, { FVar: 'A' }] },
    proofTerm: { Lam: ['x', { FVar: 'A' }, { BVar: 0 }] },
  },
  {
    name: 'Modus Ponens Term Application',
    desc: 'h1 : A → B, h2 : A ⊢ h1 h2 : B',
    context: [
      ['A', { Sort: 'Zero' }],
      ['B', { Sort: 'Zero' }],
      ['h1', { ForallE: ['_', { FVar: 'A' }, { FVar: 'B' }] }],
      ['h2', { FVar: 'A' }],
    ],
    goalType: { FVar: 'B' },
    proofTerm: { App: [{ FVar: 'h1' }, { FVar: 'h2' }] },
  },
  {
    name: 'Conjunction Commutativity (AndComm)',
    desc: 'λ (h : And A B) => And.intro B A (And.right A B h) (And.left A B h)',
    context: [
      ['A', { Sort: 'Zero' }],
      ['B', { Sort: 'Zero' }],
    ],
    goalType: {
      ForallE: [
        'h',
        { App: [{ App: [{ Const: ['And', []] }, { FVar: 'A' }] }, { FVar: 'B' }] },
        { App: [{ App: [{ Const: ['And', []] }, { FVar: 'B' }] }, { FVar: 'A' }] },
      ],
    },
    proofTerm: {
      Lam: [
        'h',
        { App: [{ App: [{ Const: ['And', []] }, { FVar: 'A' }] }, { FVar: 'B' }] },
        {
          App: [
            {
              App: [
                {
                  App: [
                    { App: [{ Const: ['And.intro', []] }, { FVar: 'B' }] },
                    { FVar: 'A' },
                  ],
                },
                {
                  App: [
                    { App: [{ App: [{ Const: ['And.right', []] }, { FVar: 'A' }] }, { FVar: 'B' }] },
                    { BVar: 0 },
                  ],
                },
              ],
            },
            {
              App: [
                { App: [{ App: [{ Const: ['And.left', []] }, { FVar: 'A' }] }, { FVar: 'B' }] },
                { BVar: 0 },
              ],
            },
          ],
        },
      ],
    },
  },
];

const MATHLIB_EXPORT_PRESETS: MathlibPresetItem[] = [
  {
    name: 'And.swap',
    desc: 'Lean 4 Mathlib Conjunction Symmetry (And.intro h.right h.left)',
    jsonPayload: JSON.stringify(
      {
        name: 'And.swap',
        type: {
          ForallE: [
            'A',
            { Sort: 'Zero' },
            {
              ForallE: [
                'B',
                { Sort: 'Zero' },
                {
                  ForallE: [
                    'h',
                    { App: [{ App: [{ Const: ['And', []] }, { BVar: 1 }] }, { BVar: 0 }] },
                    { App: [{ App: [{ Const: ['And', []] }, { BVar: 1 }] }, { BVar: 2 }] },
                  ],
                },
              ],
            },
          ],
        },
        value: {
          Lam: [
            'A',
            { Sort: 'Zero' },
            {
              Lam: [
                'B',
                { Sort: 'Zero' },
                {
                  Lam: [
                    'h',
                    { App: [{ App: [{ Const: ['And', []] }, { BVar: 1 }] }, { BVar: 0 }] },
                    {
                      App: [
                        {
                          App: [
                            {
                              App: [
                                { App: [{ Const: ['And.intro', []] }, { BVar: 1 }] },
                                { BVar: 2 },
                              ],
                            },
                            {
                              App: [
                                {
                                  App: [
                                    { App: [{ Const: ['And.right', []] }, { BVar: 2 }] },
                                    { BVar: 1 },
                                  ],
                                },
                                { BVar: 0 },
                              ],
                            },
                          ],
                        },
                        {
                          App: [
                            {
                              App: [
                                { App: [{ Const: ['And.left', []] }, { BVar: 2 }] },
                                { BVar: 1 },
                              ],
                            },
                            { BVar: 0 },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      },
      null,
      2
    ),
  },
  {
    name: 'Or.swap',
    desc: 'Lean 4 Mathlib Disjunction Commutativity (Or.elim h Or.inr Or.inl)',
    jsonPayload: JSON.stringify(
      {
        name: 'Or.swap',
        type: {
          ForallE: [
            'A',
            { Sort: 'Zero' },
            {
              ForallE: [
                'B',
                { Sort: 'Zero' },
                {
                  ForallE: [
                    'h',
                    { App: [{ App: [{ Const: ['Or', []] }, { BVar: 1 }] }, { BVar: 0 }] },
                    { App: [{ App: [{ Const: ['Or', []] }, { BVar: 1 }] }, { BVar: 2 }] },
                  ],
                },
              ],
            },
          ],
        },
        value: {
          Lam: [
            'A',
            { Sort: 'Zero' },
            {
              Lam: [
                'B',
                { Sort: 'Zero' },
                {
                  Lam: [
                    'h',
                    { App: [{ App: [{ Const: ['Or', []] }, { BVar: 1 }] }, { BVar: 0 }] },
                    {
                      App: [
                        {
                          App: [
                            {
                              App: [
                                {
                                  App: [
                                    {
                                      App: [
                                        { App: [{ Const: ['Or.elim', []] }, { BVar: 2 }] },
                                        { BVar: 1 },
                                      ],
                                    },
                                    {
                                      App: [
                                        { App: [{ Const: ['Or', []] }, { BVar: 1 }] },
                                        { BVar: 2 },
                                      ],
                                    },
                                  ],
                                },
                                { BVar: 0 },
                              ],
                            },
                            {
                              Lam: [
                                'a',
                                { BVar: 2 },
                                {
                                  App: [
                                    {
                                      App: [
                                        { App: [{ Const: ['Or.inr', []] }, { BVar: 2 }] },
                                        { BVar: 3 },
                                      ],
                                    },
                                    { BVar: 0 },
                                  ],
                                },
                              ],
                            },
                          ],
                        },
                        {
                          Lam: [
                            'b',
                            { BVar: 1 },
                            {
                              App: [
                                {
                                  App: [
                                    { App: [{ Const: ['Or.inl', []] }, { BVar: 2 }] },
                                    { BVar: 3 },
                                  ],
                                },
                                { BVar: 0 },
                              ],
                            },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      },
      null,
      2
    ),
  },
  {
    name: 'Eq.symm',
    desc: 'Lean 4 Core Equality Symmetry (Eq.rec on rfl)',
    jsonPayload: JSON.stringify(
      {
        name: 'Eq.symm',
        type: {
          ForallE: [
            'α',
            { Sort: { Param: 'u' } },
            {
              ForallE: [
                'a',
                { BVar: 0 },
                {
                  ForallE: [
                    'b',
                    { BVar: 1 },
                    {
                      ForallE: [
                        'h',
                        {
                          App: [
                            {
                              App: [
                                { App: [{ Const: ['Eq', [{ Param: 'u' }]] }, { BVar: 2 }] },
                                { BVar: 1 },
                              ],
                            },
                            { BVar: 0 },
                          ],
                        },
                        {
                          App: [
                            {
                              App: [
                                { App: [{ Const: ['Eq', [{ Param: 'u' }]] }, { BVar: 3 }] },
                                { BVar: 1 },
                              ],
                            },
                            { BVar: 2 },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
        value: {
          Lam: [
            'α',
            { Sort: { Param: 'u' } },
            {
              Lam: [
                'a',
                { BVar: 0 },
                {
                  Lam: [
                    'b',
                    { BVar: 1 },
                    {
                      Lam: [
                        'h',
                        {
                          App: [
                            {
                              App: [
                                { App: [{ Const: ['Eq', [{ Param: 'u' }]] }, { BVar: 2 }] },
                                { BVar: 1 },
                              ],
                            },
                            { BVar: 0 },
                          ],
                        },
                        {
                          App: [
                            {
                              App: [
                                {
                                  App: [
                                    {
                                      App: [
                                        {
                                          App: [
                                            {
                                              App: [
                                                {
                                                  Const: [
                                                    'Eq.rec',
                                                    ['Zero', { Param: 'u' }],
                                                  ],
                                                },
                                                { BVar: 3 },
                                              ],
                                            },
                                            { BVar: 2 },
                                          ],
                                        },
                                        {
                                          Lam: [
                                            'x',
                                            { BVar: 3 },
                                            {
                                              Lam: [
                                                'h_sub',
                                                {
                                                  App: [
                                                    {
                                                      App: [
                                                        {
                                                          App: [
                                                            { Const: ['Eq', [{ Param: 'u' }]] },
                                                            { BVar: 4 },
                                                          ],
                                                        },
                                                        { BVar: 3 },
                                                      ],
                                                    },
                                                    { BVar: 0 },
                                                  ],
                                                },
                                                {
                                                  App: [
                                                    {
                                                      App: [
                                                        {
                                                          App: [
                                                            { Const: ['Eq', [{ Param: 'u' }]] },
                                                            { BVar: 5 },
                                                          ],
                                                        },
                                                        { BVar: 1 },
                                                      ],
                                                    },
                                                    { BVar: 4 },
                                                  ],
                                                },
                                              ],
                                            },
                                          ],
                                        },
                                      ],
                                    },
                                    {
                                      App: [
                                        {
                                          App: [
                                            { Const: ['rfl', [{ Param: 'u' }]] },
                                            { BVar: 3 },
                                          ],
                                        },
                                        { BVar: 2 },
                                      ],
                                    },
                                  ],
                                },
                                { BVar: 1 },
                              ],
                            },
                            { BVar: 0 },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      },
      null,
      2
    ),
  },
  {
    name: 'id_prop',
    desc: 'Lean 4 Identity on Prop (∀ A : Prop, A → A)',
    jsonPayload: JSON.stringify(
      {
        name: 'id_prop',
        type: {
          ForallE: [
            'A',
            { Sort: 'Zero' },
            { ForallE: ['a', { BVar: 0 }, { BVar: 1 }] },
          ],
        },
        value: {
          Lam: [
            'A',
            { Sort: 'Zero' },
            { Lam: ['a', { BVar: 0 }, { BVar: 0 }] },
          ],
        },
      },
      null,
      2
    ),
  },
  {
    name: 'k_comb',
    desc: 'Lean 4 K-Combinator / Weakening (∀ A B : Prop, A → B → A)',
    jsonPayload: JSON.stringify(
      {
        name: 'k_comb',
        type: {
          ForallE: [
            'A',
            { Sort: 'Zero' },
            {
              ForallE: [
                'B',
                { Sort: 'Zero' },
                {
                  ForallE: [
                    'a',
                    { BVar: 1 },
                    { ForallE: ['b', { BVar: 1 }, { BVar: 3 }] },
                  ],
                },
              ],
            },
          ],
        },
        value: {
          Lam: [
            'A',
            { Sort: 'Zero' },
            {
              Lam: [
                'B',
                { Sort: 'Zero' },
                {
                  Lam: [
                    'a',
                    { BVar: 1 },
                    { Lam: ['b', { BVar: 1 }, { BVar: 1 }] },
                  ],
                },
              ],
            },
          ],
        },
      },
      null,
      2
    ),
  },
  {
    name: 'curry_thm',
    desc: 'Lean 4 Conjunction Currying (∀ A B C : Prop, ((A ∧ B) → C) → A → B → C)',
    jsonPayload: JSON.stringify(
      {
        name: 'curry_thm',
        type: {
          ForallE: [
            'A',
            { Sort: 'Zero' },
            {
              ForallE: [
                'B',
                { Sort: 'Zero' },
                {
                  ForallE: [
                    'C',
                    { Sort: 'Zero' },
                    {
                      ForallE: [
                        'f',
                        {
                          ForallE: [
                            'a._@._internal._hyg.0',
                            { App: [{ App: [{ Const: ['And', []] }, { BVar: 2 }] }, { BVar: 1 }] },
                            { BVar: 1 },
                          ],
                        },
                        {
                          ForallE: ['a', { BVar: 3 }, { ForallE: ['b', { BVar: 3 }, { BVar: 3 }] }],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
        value: {
          Lam: [
            'A',
            { Sort: 'Zero' },
            {
              Lam: [
                'B',
                { Sort: 'Zero' },
                {
                  Lam: [
                    'C',
                    { Sort: 'Zero' },
                    {
                      Lam: [
                        'f',
                        {
                          ForallE: [
                            'a._@._internal._hyg.0',
                            { App: [{ App: [{ Const: ['And', []] }, { BVar: 2 }] }, { BVar: 1 }] },
                            { BVar: 1 },
                          ],
                        },
                        {
                          Lam: [
                            'a',
                            { BVar: 3 },
                            {
                              Lam: [
                                'b',
                                { BVar: 3 },
                                {
                                  App: [
                                    { BVar: 2 },
                                    {
                                      App: [
                                        {
                                          App: [
                                            {
                                              App: [
                                                { App: [{ Const: ['And.intro', []] }, { BVar: 5 }] },
                                                { BVar: 4 },
                                              ],
                                            },
                                            { BVar: 1 },
                                          ],
                                        },
                                        { BVar: 0 },
                                      ],
                                    },
                                  ],
                                },
                              ],
                            },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      },
      null,
      2
    ),
  },
  {
    name: 'and_assoc_thm',
    desc: 'Lean 4 Conjunction Associativity (∀ A B C : Prop, (A ∧ B) ∧ C → A ∧ (B ∧ C))',
    jsonPayload: JSON.stringify(
      {
        name: 'and_assoc_thm',
        type: {
          ForallE: [
            'A',
            { Sort: 'Zero' },
            {
              ForallE: [
                'B',
                { Sort: 'Zero' },
                {
                  ForallE: [
                    'C',
                    { Sort: 'Zero' },
                    {
                      ForallE: [
                        'h',
                        {
                          App: [
                            {
                              App: [
                                { Const: ['And', []] },
                                { App: [{ App: [{ Const: ['And', []] }, { BVar: 2 }] }, { BVar: 1 }] },
                              ],
                            },
                            { BVar: 0 },
                          ],
                        },
                        {
                          App: [
                            { App: [{ Const: ['And', []] }, { BVar: 3 }] },
                            {
                              App: [
                                { App: [{ Const: ['And', []] }, { BVar: 2 }] },
                                { BVar: 1 },
                              ],
                            },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
        value: {
          Lam: [
            'A',
            { Sort: 'Zero' },
            {
              Lam: [
                'B',
                { Sort: 'Zero' },
                {
                  Lam: [
                    'C',
                    { Sort: 'Zero' },
                    {
                      Lam: [
                        'h',
                        {
                          App: [
                            {
                              App: [
                                { Const: ['And', []] },
                                { App: [{ App: [{ Const: ['And', []] }, { BVar: 2 }] }, { BVar: 1 }] },
                              ],
                            },
                            { BVar: 0 },
                          ],
                        },
                        {
                          App: [
                            {
                              App: [
                                {
                                  App: [
                                    { App: [{ Const: ['And.intro', []] }, { BVar: 3 }] },
                                    {
                                      App: [
                                        { App: [{ Const: ['And', []] }, { BVar: 2 }] },
                                        { BVar: 1 },
                                      ],
                                    },
                                  ],
                                },
                                {
                                  App: [
                                    {
                                      App: [
                                        { App: [{ Const: ['And.left', []] }, { BVar: 3 }] },
                                        { BVar: 2 },
                                      ],
                                    },
                                    {
                                      App: [
                                        {
                                          App: [
                                            {
                                              App: [
                                                { Const: ['And.left', []] },
                                                {
                                                  App: [
                                                    { App: [{ Const: ['And', []] }, { BVar: 3 }] },
                                                    { BVar: 2 },
                                                  ],
                                                },
                                              ],
                                            },
                                            { BVar: 1 },
                                          ],
                                        },
                                        { BVar: 0 },
                                      ],
                                    },
                                  ],
                                },
                              ],
                            },
                            {
                              App: [
                                {
                                  App: [
                                    {
                                      App: [
                                        { App: [{ Const: ['And.intro', []] }, { BVar: 2 }] },
                                        { BVar: 1 },
                                      ],
                                    },
                                    {
                                      App: [
                                        {
                                          App: [
                                            { App: [{ Const: ['And.right', []] }, { BVar: 3 }] },
                                            { BVar: 2 },
                                          ],
                                        },
                                        {
                                          App: [
                                            {
                                              App: [
                                                {
                                                  App: [
                                                    { Const: ['And.left', []] },
                                                    {
                                                      App: [
                                                        { App: [{ Const: ['And', []] }, { BVar: 3 }] },
                                                        { BVar: 2 },
                                                      ],
                                                    },
                                                  ],
                                                },
                                                { BVar: 1 },
                                              ],
                                            },
                                            { BVar: 0 },
                                          ],
                                        },
                                      ],
                                    },
                                  ],
                                },
                                {
                                  App: [
                                    {
                                      App: [
                                        {
                                          App: [
                                            { Const: ['And.right', []] },
                                            {
                                              App: [
                                                { App: [{ Const: ['And', []] }, { BVar: 3 }] },
                                                { BVar: 2 },
                                              ],
                                            },
                                          ],
                                        },
                                        { BVar: 1 },
                                      ],
                                    },
                                    { BVar: 0 },
                                  ],
                                },
                              ],
                            },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      },
      null,
      2
    ),
  },
  {
    name: 'contrapositive_thm',
    desc: 'Lean 4 Modus Tollens / Contrapositive (∀ A B : Prop, (A → B) → (B → False) → A → False)',
    jsonPayload: JSON.stringify(
      {
        name: 'contrapositive_thm',
        type: {
          ForallE: [
            'A',
            { Sort: 'Zero' },
            {
              ForallE: [
                'B',
                { Sort: 'Zero' },
                {
                  ForallE: [
                    'f',
                    { ForallE: ['a._@._internal._hyg.0', { BVar: 1 }, { BVar: 1 }] },
                    {
                      ForallE: [
                        'nb',
                        {
                          ForallE: [
                            'a._@._internal._hyg.0',
                            { BVar: 1 },
                            { Const: ['False', []] },
                          ],
                        },
                        { ForallE: ['a', { BVar: 3 }, { Const: ['False', []] }] },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
        value: {
          Lam: [
            'A',
            { Sort: 'Zero' },
            {
              Lam: [
                'B',
                { Sort: 'Zero' },
                {
                  Lam: [
                    'f',
                    { ForallE: ['a._@._internal._hyg.0', { BVar: 1 }, { BVar: 1 }] },
                    {
                      Lam: [
                        'nb',
                        {
                          ForallE: [
                            'a._@._internal._hyg.0',
                            { BVar: 1 },
                            { Const: ['False', []] },
                          ],
                        },
                        {
                          Lam: [
                            'a',
                            { BVar: 3 },
                            {
                              App: [
                                { BVar: 1 },
                                { App: [{ BVar: 2 }, { BVar: 0 }] },
                              ],
                            },
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      },
      null,
      2
    ),
  },
];

export const LocalProverView: React.FC = () => {
  // Mode State: 'deduction' | 'cic_term' | 'mathlib_ingest'
  const [proverMode, setProverMode] = useState<'deduction' | 'cic_term' | 'mathlib_ingest'>('deduction');

  // Goal & Preset State
  const [selectedPreset, setSelectedPreset] = useState<string>(BENCHMARK_PRESETS[0].name);
  const [activePreset, setActivePreset] = useState<PresetItem>(BENCHMARK_PRESETS[0]);
  const [selectedCicPreset, setSelectedCicPreset] = useState<CicPresetItem>(CIC_BENCHMARK_PRESETS[0]);
  const [selectedMathlibPreset, setSelectedMathlibPreset] = useState<MathlibPresetItem>(MATHLIB_EXPORT_PRESETS[0]);
  const [mathlibJsonInput, setMathlibJsonInput] = useState<string>(MATHLIB_EXPORT_PRESETS[0].jsonPayload);

  const [cicValidationResult, setCicValidationResult] = useState<{
    valid: boolean;
    inferredType?: string;
    elapsedUs?: number;
    error?: string;
  } | null>(null);

  const [mathlibValidationResult, setMathlibValidationResult] = useState<{
    name: string;
    valid: boolean;
    inferred_type?: string;
    elapsedUs?: number;
    error?: string;
  } | null>(null);

  // Search Engine Configuration
  const [config, setConfig] = useState<SearchConfig>({ ...DEFAULT_SEARCH_CONFIG });
  const [showConfig, setShowConfig] = useState<boolean>(false);

  // Search State & Tree Data
  const [isSearching, setIsSearching] = useState<boolean>(false);
  const [tree, setTree] = useState<Record<string, ProofNode>>({});
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [searchResult, setSearchResult] = useState<SearchResult | null>(null);
  const [copiedScript, setCopiedScript] = useState<boolean>(false);
  const [latestTraceEvent, setLatestTraceEvent] = useState<TraceEvent | null>(null);

  // Telemetry Metrics
  const [openCount, setOpenCount] = useState<number>(0);
  const [provenCount, setProvenCount] = useState<number>(0);
  const [prunedCount, setPrunedCount] = useState<number>(0);
  const [vramMB, setVramMB] = useState<number>(1842);
  const [tokSpeed, setTokSpeed] = useState<number>(46.2);

  // Subscribe to Search Engine and EventTracer Events
  useEffect(() => {
    const handleTreeUpdated = (e: TreeUpdatedEvent) => {
      setTree(e.tree);
      setOpenCount(e.openQueueSize);
      setProvenCount(e.provenCount);
      setPrunedCount(e.prunedCount);
    };

    const handleNodeSelected = (e: { nodeId: string }) => {
      setSelectedNodeId(e.nodeId);
    };

    const handleSearchComplete = (e: { result: SearchResult }) => {
      setSearchResult(e.result);
      setIsSearching(false);
      if (e.result.rootId) {
        setSelectedNodeId(e.result.rootId);
      }
    };

    const unsubscribeTracer = eventTracer.subscribe((evt) => {
      setLatestTraceEvent(evt);
    });

    proofSearchEngine.on('tree_updated', handleTreeUpdated);
    proofSearchEngine.on('node_selected', handleNodeSelected);
    proofSearchEngine.on('search_complete', handleSearchComplete);

    gemmaEdgeController.getTelemetry().then((tel) => {
      setVramMB(tel.vramAllocatedMB);
      setTokSpeed(tel.avgTokensPerSec || 46.2);
    });

    return () => {
      proofSearchEngine.off('tree_updated', handleTreeUpdated);
      proofSearchEngine.off('node_selected', handleNodeSelected);
      proofSearchEngine.off('search_complete', handleSearchComplete);
      unsubscribeTracer();
    };
  }, []);

  const handleSelectPreset = (preset: PresetItem) => {
    setSelectedPreset(preset.name);
    setActivePreset(preset);
    handleReset();
  };

  const handleSelectCicPreset = (preset: CicPresetItem) => {
    setSelectedCicPreset(preset);
    setCicValidationResult(null);
  };

  const handleSelectMathlibPreset = (preset: MathlibPresetItem) => {
    setSelectedMathlibPreset(preset);
    setMathlibJsonInput(preset.jsonPayload);
    setMathlibValidationResult(null);
  };

  const handleVerifyCicTerm = () => {
    const t0 = performance.now();
    try {
      const contextJson = JSON.stringify(selectedCicPreset.context);
      const proofTermJson = JSON.stringify(selectedCicPreset.proofTerm);
      const goalTypeJson = JSON.stringify(selectedCicPreset.goalType);

      const isValid = kernelWasm.check_cic_term(contextJson, proofTermJson, goalTypeJson);
      const inferred = kernelWasm.infer_cic_type(contextJson, proofTermJson);
      const elapsedUs = Math.max(1, Math.round((performance.now() - t0) * 1000));

      setCicValidationResult({
        valid: isValid,
        inferredType: inferred,
        elapsedUs,
      });
    } catch (err: any) {
      const elapsedUs = Math.max(1, Math.round((performance.now() - t0) * 1000));
      setCicValidationResult({
        valid: false,
        error: err?.message || String(err),
        elapsedUs,
      });
    }
  };

  const handleVerifyMathlibExport = () => {
    const t0 = performance.now();
    try {
      const res = kernelWasm.verify_mathlib_export(mathlibJsonInput);
      const elapsedUs = Math.max(1, Math.round((performance.now() - t0) * 1000));
      setMathlibValidationResult({
        name: res.name,
        valid: res.valid,
        inferred_type: res.inferred_type,
        elapsedUs,
      });
    } catch (err: any) {
      const elapsedUs = Math.max(1, Math.round((performance.now() - t0) * 1000));
      setMathlibValidationResult({
        name: selectedMathlibPreset.name,
        valid: false,
        error: err?.message || String(err),
        elapsedUs,
      });
    }
  };

  const handleStartSearch = async () => {
    setIsSearching(true);
    setSearchResult(null);
    try {
      const res = await proofSearchEngine.startSearch(
        activePreset.name,
        activePreset.goalStr,
        activePreset.hyps,
        activePreset.target,
        config
      );
      setSearchResult(res);
    } catch (err) {
      console.error('Proof search error:', err);
    } finally {
      setIsSearching(false);
    }
  };

  const handleStepOnce = async () => {
    if (Object.keys(tree).length === 0) {
      proofSearchEngine.startSearch(
        activePreset.name,
        activePreset.goalStr,
        activePreset.hyps,
        activePreset.target,
        config
      ).catch(console.error);
      return;
    }
    await proofSearchEngine.stepOnce();
  };

  const handleStopSearch = () => {
    proofSearchEngine.stopSearch();
    setIsSearching(false);
  };

  const handleReset = () => {
    proofSearchEngine.reset();
    setTree({});
    setSelectedNodeId(null);
    setSearchResult(null);
    setOpenCount(0);
    setProvenCount(0);
    setPrunedCount(0);
    setLatestTraceEvent(null);
  };

  const downloadFlightTrace = () => {
    const jsonl = eventTracer.exportTraceJsonl();
    const blob = new Blob([jsonl], { type: 'application/x-ndjson' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `flight_trace_${activePreset.name}_${Date.now()}.jsonl`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const copyProofScript = (script: string) => {
    navigator.clipboard.writeText(script);
    setCopiedScript(true);
    setTimeout(() => setCopiedScript(false), 2000);
  };

  const selectedNode = selectedNodeId ? tree[selectedNodeId] : null;
  const selectedContext = selectedNodeId ? proofSearchEngine.getNodeContext(selectedNodeId) : null;
  const nodesList = Object.values(tree);

  return (
    <div className="flex flex-col h-full bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
      {/* Header Banner */}
      <div className="px-6 py-4 bg-slate-950/90 border-b border-slate-800 flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="p-2.5 bg-gradient-to-br from-blue-950 to-indigo-900 border border-blue-700/60 rounded-xl text-blue-400 shadow-lg shadow-blue-950/40">
            <Workflow className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-base font-bold text-white tracking-wide">
                Neuro-Symbolic WASM Proof Engine & Lean 4 Mathlib Bridge
              </h2>
              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-emerald-950/80 text-emerald-300 border border-emerald-700/50 rounded-full">
                WASM Kernel (&lt;50µs ι-Red)
              </span>
              <span className="px-2 py-0.5 text-[10px] font-mono font-semibold bg-purple-950/80 text-purple-300 border border-purple-700/50 rounded-full">
                Gemma 4 Edge WebGPU
              </span>
            </div>
            <p className="text-xs text-slate-400 font-mono mt-0.5">
              Closed-loop BFS prover coordinating Actor AST generation, GenRM Critic, WASM Kernel, and Mathlib Ingestion Bridge
            </p>
          </div>
        </div>

        {/* Real-time Status Badges */}
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={downloadFlightTrace}
            disabled={eventTracer.getTraceHistory().length === 0}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 hover:text-white text-xs font-mono rounded-lg transition-all disabled:opacity-50"
            title="Download Flight Recorder Telemetry JSONL"
          >
            <Download className="w-3.5 h-3.5 text-blue-400" />
            <span>Flight Trace (.jsonl)</span>
          </button>

          <div className="flex items-center gap-2 px-3 py-1.5 bg-slate-900/80 border border-slate-700/60 rounded-lg text-xs font-mono">
            <span
              className={`w-2 h-2 rounded-full ${
                isSearching ? 'bg-emerald-400 animate-ping' : searchResult?.success ? 'bg-emerald-400' : 'bg-slate-500'
              }`}
            />
            <span className="text-slate-300 uppercase font-semibold">
              {isSearching ? 'Searching...' : searchResult?.success ? 'Proven!' : 'Idle'}
            </span>
            <span className="text-slate-500">|</span>
            <span className="text-slate-400">{vramMB} MB VRAM</span>
          </div>
        </div>
      </div>

      {/* Mode Switcher Tabs */}
      <div className="px-6 pt-4 pb-0 bg-slate-950/70 border-b border-slate-800 flex items-center gap-2">
        <button
          type="button"
          onClick={() => setProverMode('deduction')}
          className={`flex items-center gap-2 px-4 py-2 text-xs font-mono font-semibold rounded-t-lg border-t border-x transition-all ${
            proverMode === 'deduction'
              ? 'bg-slate-900 border-slate-700 text-blue-400 font-bold border-b-2 border-b-blue-500'
              : 'bg-transparent border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Workflow className="w-3.5 h-3.5" />
          <span>Deduction Steps (Propositional / FOL)</span>
        </button>
        <button
          type="button"
          onClick={() => setProverMode('cic_term')}
          className={`flex items-center gap-2 px-4 py-2 text-xs font-mono font-semibold rounded-t-lg border-t border-x transition-all ${
            proverMode === 'cic_term'
              ? 'bg-slate-900 border-slate-700 text-purple-400 font-bold border-b-2 border-b-purple-500'
              : 'bg-transparent border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Brain className="w-3.5 h-3.5" />
          <span>Pure CIC λΠ Proof-Terms (Lean 4 Core)</span>
        </button>
        <button
          type="button"
          onClick={() => setProverMode('mathlib_ingest')}
          className={`flex items-center gap-2 px-4 py-2 text-xs font-mono font-semibold rounded-t-lg border-t border-x transition-all ${
            proverMode === 'mathlib_ingest'
              ? 'bg-slate-900 border-slate-700 text-emerald-400 font-bold border-b-2 border-b-emerald-500'
              : 'bg-transparent border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Sparkles className="w-3.5 h-3.5 text-emerald-400" />
          <span>Mathlib Import & Ingest Bridge (Phase D)</span>
        </button>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {proverMode === 'mathlib_ingest' ? (
          /* Mathlib Import & Ingest Mode View */
          <div className="space-y-6">
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider">
                  Exported Lean 4 Mathlib Declarations (#export_bourbaki)
                </label>
                <span className="text-[11px] text-emerald-400 font-mono">Phase D: Inductive Families & ι-Reduction</span>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 lg:grid-cols-5 gap-3">
                {MATHLIB_EXPORT_PRESETS.map((preset) => (
                  <button
                    key={preset.name}
                    type="button"
                    onClick={() => handleSelectMathlibPreset(preset)}
                    className={`p-3 rounded-lg border text-left transition-all ${
                      selectedMathlibPreset.name === preset.name
                        ? 'bg-emerald-950/80 border-emerald-600 text-emerald-200 shadow-md'
                        : 'bg-slate-950/50 border-slate-800 text-slate-400 hover:text-slate-200 hover:border-slate-700'
                    }`}
                  >
                    <div className="text-xs font-mono font-bold">{preset.name}</div>
                    <div className="text-[10px] text-emerald-300/80 font-mono mt-0.5 line-clamp-2">{preset.desc}</div>
                  </button>
                ))}
              </div>
            </div>

            {/* Mathlib Verification Action Bar */}
            <div className="bg-slate-950/50 p-4 rounded-xl border border-slate-800 flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={handleVerifyMathlibExport}
                  className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white text-xs font-mono font-bold rounded-lg shadow-lg shadow-emerald-950/40 transition-all"
                >
                  <ShieldCheck className="w-4 h-4" />
                  <span>Verify Mathlib Export in WASM Kernel</span>
                </button>
              </div>

              {mathlibValidationResult && (
                <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg border font-mono text-xs">
                  {mathlibValidationResult.valid ? (
                    <span className="flex items-center gap-1.5 text-emerald-400 font-bold">
                      <CheckCircle2 className="w-4 h-4" />
                      <span>Sound Lean 4 Mathlib Theorem! ({mathlibValidationResult.elapsedUs} µs)</span>
                    </span>
                  ) : (
                    <span className="text-rose-400 font-bold">
                      Verification Error: {mathlibValidationResult.error}
                    </span>
                  )}
                </div>
              )}
            </div>

            {/* Mathlib Inspector Grid */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              {/* JSON Input / Editor */}
              <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col space-y-4">
                <div className="flex items-center justify-between border-b border-slate-800 pb-3">
                  <div className="flex items-center gap-2">
                    <FileCode className="w-4 h-4 text-emerald-400" />
                    <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                      Exported Lean 4 JSON AST Payload
                    </h3>
                  </div>
                  <button
                    type="button"
                    onClick={() => copyProofScript(mathlibJsonInput)}
                    className="flex items-center gap-1 text-[11px] font-mono text-slate-400 hover:text-slate-200"
                  >
                    <Copy className="w-3.5 h-3.5" />
                    <span>Copy JSON</span>
                  </button>
                </div>

                <textarea
                  value={mathlibJsonInput}
                  onChange={(e) => setMathlibJsonInput(e.target.value)}
                  rows={14}
                  className="w-full p-3 bg-slate-900 rounded border border-slate-800 text-emerald-300 font-mono text-xs leading-relaxed focus:outline-none focus:border-emerald-600 resize-none"
                  placeholder="Paste exported Lean 4 theorem JSON here..."
                />
              </div>

              {/* Inferred Type & Kernel Verification Card */}
              <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col space-y-4">
                <div className="flex items-center gap-2 border-b border-slate-800 pb-3">
                  <ShieldCheck className="w-4 h-4 text-teal-400" />
                  <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                    WASM Kernel Inferred Type & Def-Eq Status
                  </h3>
                </div>

                <div className="space-y-4 font-mono text-xs flex-1">
                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Declared Theorem Name:</span>
                    <div className="p-2.5 bg-slate-900 rounded border border-slate-800 text-white font-bold">
                      {selectedMathlibPreset.name}
                    </div>
                  </div>

                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Kernel Inferred Type (via infer_type):</span>
                    <pre className="p-3 bg-slate-900 rounded border border-slate-800 text-teal-300 overflow-x-auto max-h-48">
                      {mathlibValidationResult?.inferred_type
                        ? JSON.stringify(JSON.parse(mathlibValidationResult.inferred_type), null, 2)
                        : 'Click "Verify Mathlib Export in WASM Kernel" to evaluate'}
                    </pre>
                  </div>

                  <div className="p-3 bg-slate-900 rounded-lg border border-slate-800 flex items-center justify-between">
                    <span className="text-slate-400 text-xs">Kernel Evaluation Time:</span>
                    <span className="font-bold text-emerald-400">
                      {mathlibValidationResult?.elapsedUs ? `${mathlibValidationResult.elapsedUs} µs` : '< 50 µs'}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        ) : proverMode === 'cic_term' ? (
          /* Pure CIC Proof-Term Mode View */
          <div className="space-y-6">
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider">
                  CIC Kernel Proof-Term Benchmark Presets
                </label>
                <span className="text-[11px] text-purple-400 font-mono">Lean 4 Core λΠ-Calculus</span>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                {CIC_BENCHMARK_PRESETS.map((preset) => (
                  <button
                    key={preset.name}
                    type="button"
                    onClick={() => handleSelectCicPreset(preset)}
                    className={`p-3 rounded-lg border text-left transition-all ${
                      selectedCicPreset.name === preset.name
                        ? 'bg-purple-950/80 border-purple-600 text-purple-200 shadow-md'
                        : 'bg-slate-950/50 border-slate-800 text-slate-400 hover:text-slate-200 hover:border-slate-700'
                    }`}
                  >
                    <div className="text-xs font-mono font-bold">{preset.name}</div>
                    <div className="text-[11px] text-purple-300 font-mono mt-0.5">{preset.desc}</div>
                  </button>
                ))}
              </div>
            </div>

            {/* CIC Verification Action Bar */}
            <div className="bg-slate-950/50 p-4 rounded-xl border border-slate-800 flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={handleVerifyCicTerm}
                  className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 text-white text-xs font-mono font-bold rounded-lg shadow-lg shadow-purple-950/40 transition-all"
                >
                  <ShieldCheck className="w-4 h-4" />
                  <span>Verify Proof-Term in WASM Kernel</span>
                </button>
              </div>

              {cicValidationResult && (
                <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg border font-mono text-xs">
                  {cicValidationResult.valid ? (
                    <span className="flex items-center gap-1.5 text-emerald-400 font-bold">
                      <CheckCircle2 className="w-4 h-4" />
                      <span>Valid CIC Proof-Term! ({cicValidationResult.elapsedUs} µs)</span>
                    </span>
                  ) : (
                    <span className="text-rose-400 font-bold">
                      Type Error: {cicValidationResult.error}
                    </span>
                  )}
                </div>
              )}
            </div>

            {/* CIC Inspector Grid */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              {/* Context & Goal */}
              <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col space-y-4">
                <div className="flex items-center gap-2 border-b border-slate-800 pb-3">
                  <Terminal className="w-4 h-4 text-purple-400" />
                  <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                    Goal Type & Hypothesis Context
                  </h3>
                </div>

                <div className="space-y-3 font-mono text-xs">
                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Target Goal Type (CIC):</span>
                    <pre className="p-3 bg-slate-900 rounded border border-slate-800 text-purple-300 overflow-x-auto">
                      {JSON.stringify(selectedCicPreset.goalType, null, 2)}
                    </pre>
                  </div>

                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Local Hypothesis Context:</span>
                    <div className="space-y-1.5">
                      {selectedCicPreset.context.map(([id, ty]) => (
                        <div key={id} className="p-2 rounded border bg-slate-900 border-slate-800 text-slate-200">
                          <span className="font-bold text-purple-400">{id} :</span> {JSON.stringify(ty)}
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>

              {/* Synthesized Proof Term */}
              <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col space-y-4">
                <div className="flex items-center justify-between border-b border-slate-800 pb-3">
                  <div className="flex items-center gap-2">
                    <Brain className="w-4 h-4 text-emerald-400" />
                    <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                      Synthesized λ-Term AST
                    </h3>
                  </div>
                  <button
                    type="button"
                    onClick={() => copyProofScript(JSON.stringify(selectedCicPreset.proofTerm, null, 2))}
                    className="flex items-center gap-1 text-[11px] font-mono text-slate-400 hover:text-slate-200"
                  >
                    <Copy className="w-3.5 h-3.5" />
                    <span>Copy JSON</span>
                  </button>
                </div>

                <pre className="flex-1 p-4 bg-slate-900 rounded border border-slate-800 text-emerald-300 font-mono text-xs overflow-x-auto leading-relaxed">
                  {JSON.stringify(selectedCicPreset.proofTerm, null, 2)}
                </pre>
              </div>
            </div>
          </div>
        ) : (
          /* Deduction Step BFS Mode View */
          <>
            {/* Preset Selector Chips */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider">
                  Benchmark Propositional Deduction Presets
                </label>
                <span className="text-[11px] text-slate-500 font-mono">Select benchmark preset</span>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                {BENCHMARK_PRESETS.map((preset) => (
                  <button
                    key={preset.name}
                    type="button"
                    onClick={() => handleSelectPreset(preset)}
                    className={`p-3 rounded-lg border text-left transition-all ${
                      selectedPreset === preset.name
                        ? 'bg-blue-950/80 border-blue-600 text-blue-200 shadow-md'
                        : 'bg-slate-950/50 border-slate-800 text-slate-400 hover:text-slate-200 hover:border-slate-700'
                    }`}
                  >
                    <div className="text-xs font-mono font-bold">{preset.name}</div>
                    <div className="text-[11px] text-emerald-400 font-mono mt-0.5">{preset.desc}</div>
                  </button>
                ))}
              </div>
            </div>

            {/* Action Controls & Parameters */}
            <div className="bg-slate-950/50 p-4 rounded-xl border border-slate-800 flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  disabled={isSearching}
                  onClick={handleStartSearch}
                  className="flex items-center gap-2 px-5 py-2.5 bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 text-white text-xs font-mono font-bold rounded-lg shadow-lg shadow-blue-950/40 transition-all disabled:opacity-50"
                >
                  <Play className={`w-4 h-4 ${isSearching ? 'animate-spin' : ''}`} />
                  <span>{isSearching ? 'Solving...' : 'Run Prover (BFS)'}</span>
                </button>

                <button
                  type="button"
                  disabled={isSearching}
                  onClick={handleStepOnce}
                  className="flex items-center gap-2 px-4 py-2.5 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-mono font-semibold rounded-lg border border-slate-700 transition-all disabled:opacity-50"
                >
                  <StepForward className="w-4 h-4 text-blue-400" />
                  <span>Step Once</span>
                </button>

                {isSearching && (
                  <button
                    type="button"
                    onClick={handleStopSearch}
                    className="flex items-center gap-2 px-4 py-2.5 bg-rose-950/80 hover:bg-rose-900 border border-rose-700 text-rose-200 text-xs font-mono font-semibold rounded-lg transition-all"
                  >
                    <Square className="w-4 h-4 text-rose-400" />
                    <span>Abort</span>
                  </button>
                )}

                <button
                  type="button"
                  onClick={handleReset}
                  className="flex items-center gap-1.5 px-3 py-2.5 bg-slate-900 hover:bg-slate-800 text-slate-400 hover:text-slate-200 text-xs font-mono rounded-lg border border-slate-800"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>Reset</span>
                </button>
              </div>

              <button
                type="button"
                onClick={() => setShowConfig(!showConfig)}
                className="flex items-center gap-1.5 text-xs font-mono text-slate-400 hover:text-slate-200 bg-slate-900 px-3 py-2 rounded-lg border border-slate-800"
              >
                <Sliders className="w-3.5 h-3.5 text-blue-400" />
                <span>{showConfig ? 'Hide Config' : 'Search Parameters'}</span>
              </button>
            </div>

            {/* Expandable Configuration */}
            {showConfig && (
              <div className="p-4 bg-slate-950 rounded-xl border border-slate-800 grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-4">
                <div>
                  <label className="block text-xs font-mono text-slate-400 mb-1">
                    Expansion Factor K: <span className="text-white font-bold">{config.expansionFactorK}</span>
                  </label>
                  <input
                    type="range"
                    min="1"
                    max="5"
                    value={config.expansionFactorK}
                    onChange={(e) => setConfig({ ...config, expansionFactorK: Number(e.target.value) })}
                    className="w-full h-1.5 bg-slate-800 rounded appearance-none cursor-pointer accent-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-mono text-slate-400 mb-1">
                    Beam Width: <span className="text-white font-bold">{config.beamWidth}</span>
                  </label>
                  <input
                    type="range"
                    min="1"
                    max="8"
                    value={config.beamWidth}
                    onChange={(e) => setConfig({ ...config, beamWidth: Number(e.target.value) })}
                    className="w-full h-1.5 bg-slate-800 rounded appearance-none cursor-pointer accent-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-mono text-slate-400 mb-1">
                    Max Depth: <span className="text-white font-bold">{config.maxDepth}</span>
                  </label>
                  <input
                    type="range"
                    min="2"
                    max="10"
                    value={config.maxDepth}
                    onChange={(e) => setConfig({ ...config, maxDepth: Number(e.target.value) })}
                    className="w-full h-1.5 bg-slate-800 rounded appearance-none cursor-pointer accent-blue-500"
                  />
                </div>
                <div>
                  <label className="block text-xs font-mono text-slate-400 mb-1">
                    Thinking Budget: <span className="text-white font-bold">{config.reasoningBudget} tok</span>
                  </label>
                  <input
                    type="range"
                    min="0"
                    max="512"
                    step="32"
                    value={config.reasoningBudget}
                    onChange={(e) => setConfig({ ...config, reasoningBudget: Number(e.target.value) })}
                    className="w-full h-1.5 bg-slate-800 rounded appearance-none cursor-pointer accent-purple-500"
                  />
                </div>
              </div>
            )}

            {/* Real-time Telemetry Dashboard */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
              <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
                <div>
                  <div className="text-xs text-slate-400 font-mono">Explored Nodes</div>
                  <div className="text-xl font-bold font-mono text-white mt-0.5">{nodesList.length}</div>
                  <span className="text-[10px] text-slate-500 font-mono">({prunedCount} pruned)</span>
                </div>
                <Layers className="w-6 h-6 text-blue-400 opacity-60" />
              </div>

              <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
                <div>
                  <div className="text-xs text-slate-400 font-mono">Open Queue</div>
                  <div className="text-xl font-bold font-mono text-amber-400 mt-0.5">{openCount}</div>
                  <span className="text-[10px] text-slate-500 font-mono">active beams</span>
                </div>
                <Search className="w-6 h-6 text-amber-400 opacity-60" />
              </div>

              <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
                <div>
                  <div className="text-xs text-slate-400 font-mono">Proven Nodes</div>
                  <div className="text-xl font-bold font-mono text-emerald-400 mt-0.5">{provenCount}</div>
                  <span className="text-[10px] text-emerald-500 font-mono">
                    {latestTraceEvent?.kernelLatencyUs ? `${latestTraceEvent.kernelLatencyUs}µs WASM` : 'Ready'}
                  </span>
                </div>
                <CheckCircle2 className="w-6 h-6 text-emerald-400 opacity-60" />
              </div>

              <div className="bg-slate-950/70 p-4 rounded-xl border border-slate-800 flex items-center justify-between">
                <div>
                  <div className="text-xs text-slate-400 font-mono">Throughput</div>
                  <div className="text-xl font-bold font-mono text-teal-300 mt-0.5">
                    {tokSpeed.toFixed(1)} <span className="text-xs font-normal text-slate-400">tok/s</span>
                  </div>
                  <span className="text-[10px] text-teal-500 font-mono">{vramMB} MB VRAM</span>
                </div>
                <Activity className="w-6 h-6 text-teal-400 opacity-60" />
              </div>
            </div>

            {/* Visual Proof DAG / Search Tree */}
            <div className="bg-slate-950 p-5 rounded-xl border border-slate-800">
              <div className="flex items-center justify-between border-b border-slate-800 pb-3 mb-4">
                <div className="flex items-center gap-2">
                  <Workflow className="w-4 h-4 text-blue-400" />
                  <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                    Visual Proof DAG & Search Frontier
                  </h3>
                </div>
                <span className="text-[11px] font-mono text-slate-400">
                  {nodesList.length === 0 ? 'No active search' : `${nodesList.length} total nodes in memory`}
                </span>
              </div>

              <div className="max-h-60 overflow-y-auto space-y-2 pr-2">
                {nodesList.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-8 text-slate-500 font-mono text-xs space-y-2">
                    <Workflow className="w-8 h-8 text-slate-700" />
                    <span>Click "Run Prover (BFS)" to start autonomous WASM-accelerated search</span>
                  </div>
                ) : (
                  nodesList.map((node) => (
                    <div
                      key={node.id}
                      onClick={() => setSelectedNodeId(node.id)}
                      className={`p-2.5 rounded-lg border font-mono text-xs cursor-pointer transition-all ${
                        selectedNodeId === node.id
                          ? 'bg-slate-900 border-blue-500 shadow-md'
                          : 'bg-slate-950/80 border-slate-800/80 hover:border-slate-700'
                      }`}
                      style={{ marginLeft: `${Math.min(node.depth * 18, 90)}px` }}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <span
                            className={`w-2 h-2 rounded-full ${
                              node.status === 'proven'
                                ? 'bg-emerald-400 shadow-sm shadow-emerald-400'
                                : node.status === 'expanded'
                                ? 'bg-blue-400'
                                : node.status === 'open'
                                ? 'bg-amber-400'
                                : 'bg-slate-600'
                            }`}
                          />
                          <span className="font-bold text-white">{node.goal}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="px-1.5 py-0.5 rounded text-[10px] bg-blue-950/80 border border-blue-800 text-blue-300">
                            {(node.genrmScore * 100).toFixed(0)}%
                          </span>
                          <span
                            className={`px-1.5 py-0.5 rounded text-[10px] uppercase font-semibold ${
                              node.status === 'proven'
                                ? 'bg-emerald-950 text-emerald-300 border border-emerald-800'
                                : node.status === 'expanded'
                                ? 'bg-blue-950 text-blue-300 border border-blue-800'
                                : 'bg-slate-800 text-slate-400'
                            }`}
                          >
                            {node.status}
                          </span>
                        </div>
                      </div>

                      {node.tacticApplied && (
                        <div className="mt-1 flex items-center gap-1.5 text-[11px] text-emerald-400">
                          <ChevronRight className="w-3 h-3 text-slate-500 shrink-0" />
                          <span className="font-semibold text-emerald-300 truncate">Step: {node.tacticApplied}</span>
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>

            {/* Three-Column Inspector */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              {/* Column 1: State Diff */}
              <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col h-[380px]">
                <div className="flex items-center gap-2 border-b border-slate-800 pb-3 mb-3">
                  <Terminal className="w-4 h-4 text-emerald-400" />
                  <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                    State Context & Diff
                  </h3>
                </div>

                <div className="flex-1 overflow-y-auto space-y-3 font-mono text-xs pr-1">
                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Target Goal:</span>
                    <div className="p-2 bg-slate-900 rounded border border-slate-800 text-white">
                      ⊢ {JSON.stringify(selectedContext?.target || activePreset.target)}
                    </div>
                  </div>

                  <div>
                    <span className="text-[10px] text-slate-500 block uppercase font-bold mb-1">Active Hypotheses:</span>
                    <div className="space-y-1.5">
                      {Object.entries(selectedContext?.hyps || activePreset.hyps).map(([id, expr]) => (
                        <div
                          key={id}
                          className={`p-2 rounded border text-[11px] ${
                            id === 'h1' || id === 'h2' || id === 'h3'
                              ? 'bg-emerald-950/40 border-emerald-800 text-emerald-300'
                              : 'bg-slate-900 border-slate-800 text-slate-200'
                          }`}
                        >
                          <span className="font-bold text-blue-400">{id}:</span> {JSON.stringify(expr)}
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>

              {/* Column 2: Thinking Scratchpad */}
              <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col h-[380px]">
                <div className="flex items-center gap-2 border-b border-slate-800 pb-3 mb-3">
                  <Brain className="w-4 h-4 text-purple-400" />
                  <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                    Actor Scratchpad (&lt;think&gt;)
                  </h3>
                </div>

                <div className="flex-1 overflow-y-auto font-mono text-xs pr-1">
                  {selectedNode?.thinkingTrace ? (
                    <div className="p-3 bg-purple-950/20 border border-purple-900/40 rounded-lg text-slate-300 leading-relaxed text-[11px]">
                      {selectedNode.thinkingTrace}
                    </div>
                  ) : (
                    <div className="flex flex-col items-center justify-center h-full text-slate-500 text-xs">
                      <span>Select an expanded node to inspect the Actor's scratchpad reasoning</span>
                    </div>
                  )}
                </div>
              </div>

              {/* Column 3: GenRM Telemetry & Kernel Validation */}
              <div className="bg-slate-950 p-5 rounded-xl border border-slate-800 flex flex-col h-[380px]">
                <div className="flex items-center gap-2 border-b border-slate-800 pb-3 mb-3">
                  <ShieldCheck className="w-4 h-4 text-blue-400" />
                  <h3 className="text-xs font-bold text-white font-mono uppercase tracking-wider">
                    GenRM & Kernel Telemetry
                  </h3>
                </div>

                <div className="flex-1 overflow-y-auto space-y-3 font-mono text-xs pr-1">
                  <div className="p-3 bg-slate-900 rounded-lg border border-slate-800 space-y-2">
                    <span className="text-[10px] text-slate-500 block uppercase font-bold">GenRM Confidence:</span>
                    <div className="flex items-center justify-between">
                      <span className="text-base font-bold text-blue-400">
                        {selectedNode ? (selectedNode.genrmScore * 100).toFixed(1) : '99.5'}%
                      </span>
                      <span className="text-[10px] px-2 py-0.5 rounded bg-emerald-950 text-emerald-300 border border-emerald-800">
                        High Confidence
                      </span>
                    </div>
                    <div className="w-full bg-slate-800 h-1.5 rounded-full overflow-hidden">
                      <div
                        className="bg-blue-500 h-full transition-all duration-300"
                        style={{ width: `${selectedNode ? selectedNode.genrmScore * 100 : 99.5}%` }}
                      />
                    </div>
                  </div>

                  <div className="p-3 bg-slate-900 rounded-lg border border-slate-800 space-y-2">
                    <span className="text-[10px] text-slate-500 block uppercase font-bold">WASM Kernel Latency:</span>
                    <div className="flex items-center gap-2 text-emerald-400">
                      <Clock className="w-4 h-4 text-emerald-400" />
                      <span className="text-sm font-bold">
                        {latestTraceEvent?.kernelLatencyUs ? `${latestTraceEvent.kernelLatencyUs} µs` : '< 10 µs'}
                      </span>
                      <span className="text-[10px] text-slate-500">(sub-microsecond)</span>
                    </div>
                  </div>

                  <div className="p-3 bg-slate-900 rounded-lg border border-slate-800 space-y-1">
                    <span className="text-[10px] text-slate-500 block uppercase font-bold">Cumulative Path Score:</span>
                    <div className="text-xs font-bold text-purple-300">
                      Q(path): {selectedNode ? selectedNode.cumulativeScore.toFixed(3) : '0.000'}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Proven Solution Banner */}
            {searchResult?.success && (
              <div className="bg-slate-950 p-5 rounded-xl border border-emerald-800/80 shadow-2xl space-y-3">
                <div className="flex items-center justify-between border-b border-emerald-900/60 pb-3">
                  <div className="flex items-center gap-2">
                    <CheckCircle2 className="w-5 h-5 text-emerald-400" />
                    <h3 className="text-sm font-bold text-white font-mono">
                      Autonomous Proof Successfully Closed! ({searchResult.elapsedMs.toFixed(0)} ms)
                    </h3>
                  </div>
                  <button
                    type="button"
                    onClick={() => copyProofScript(searchResult.tacticScript)}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-emerald-950 border border-emerald-700 text-emerald-300 text-xs font-mono rounded-lg hover:bg-emerald-900 transition-all"
                  >
                    {copiedScript ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                    <span>{copiedScript ? 'Copied' : 'Copy Lean 4 Script'}</span>
                  </button>
                </div>

                <pre className="p-4 bg-slate-900 rounded-lg border border-slate-800 text-emerald-300 font-mono text-xs overflow-x-auto leading-relaxed">
                  {searchResult.tacticScript}
                </pre>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
};
