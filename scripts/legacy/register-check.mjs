#!/usr/bin/env node
// register-check.mjs — v1.1 Task 1
//
// Verifies crates/epiora-lang/register.json against the vendored Epi Info 7
// reference sources (Apache-2.0, commit 4cd353c — see reference/NOTICE):
//
//   * every command keyword derivable from the two GOLD grammars'
//     statement productions appears in register.json "commands" exactly once;
//   * every function name in the two EpiFunctions.cs dispatch tables, plus
//     every function-name terminal in the grammars' <*FuncName*> productions,
//     appears in "functions" exactly once;
//   * every member of Epi.Core's MetaFieldType enum appears in
//     "field_types" exactly once;
//   * no name is duplicated inside a section;
//   * status is supported|planned|not_planned; not_planned requires
//     version null and a non-empty reason; every other version is
//     a "X.Y" string <= "1.6" (spec: every command has a target no
//     later than v1.6 unless it is not planned, with a reason).
//
// Exit 0 on success, 1 with every violation listed on stdout.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const REF = join(root, 'crates', 'epiora-lang', 'reference');
const REGISTER = join(root, 'crates', 'epiora-lang', 'register.json');

const GRAMMARS = [
  {
    name: 'analysis',
    path: join(REF, 'Epi.Core.Interpreter', 'grammar', 'EpiInfo.Analysis.Grammar.grm'),
    // Nonterminals whose alternatives enumerate the top-level commands.
    entryPoints: ['Statement'],
  },
  {
    name: 'enter',
    path: join(REF, 'Epi.Core.EnterInterpreter', 'grammar', 'EpiInfo.Enter.Grammar.grm'),
    // Check-code commands live under <Statement>; block-level commands
    // (FIELD, PAGE, VIEW/FORM, RECORD, DEFINEVARIABLES, SUB) under
    // <CheckCodeBlock>.
    entryPoints: ['Statement', 'CheckCodeBlock'],
  },
];

const FUNCTION_DISPATCH = [
  join(REF, 'Epi.Core.Interpreter', 'Rules', 'EpiFunctions.cs'),
  join(REF, 'Epi.Core.EnterInterpreter', 'Rules', 'EpiFunctions.cs'),
];

const FIELD_TYPE_ENUM = join(REF, 'Epi.Core', 'Constants', 'MetaFieldType.cs');

// Bare-word grammar tokens that are terminal names, not command keywords.
const TERMINALS = new Set([
  'identifier', 'file', 'string', 'url', 'bracestring', 'charliteral',
  'decliteral', 'realliteral', 'hexliteral', 'percentage', 'boolean',
  'date', 'time', 'eof', 'comment_chars', 'programming_chars', 'words',
  'excelidentifier', 'excelrange',
]);

// Statement productions that produce no command keyword.
const NON_COMMAND = new Set(['Comment_Line']);

// Statement productions whose command name cannot be read from a keyword.
const NAME_OVERRIDES = new Map([
  // <Simple_Assign_Statement> ::= Identifier '=' <Expression>
  ['Simple_Assign_Statement', 'ASSIGN'],
]);

const KEYWORD_RE = /^[A-Za-z][A-Za-z0-9_-]*$/;

const failures = [];
const fail = (msg) => failures.push(msg);

// Strip GOLD '!' comments while honouring single-quoted literals.
function stripComment(line) {
  let inQuote = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === "'") inQuote = !inQuote;
    else if (c === '!' && !inQuote) return line.slice(0, i);
  }
  return line;
}

// Split an RHS chunk into alternatives on '|' outside single quotes.
function splitAlternatives(rhs) {
  const alts = [];
  let inQuote = false;
  let cur = '';
  for (const c of rhs) {
    if (c === "'") inQuote = !inQuote;
    if (c === '|' && !inQuote) {
      alts.push(cur);
      cur = '';
    } else {
      cur += c;
    }
  }
  alts.push(cur);
  return alts.map((a) => a.trim()).filter(Boolean);
}

// Parse a .grm file into { lhs -> [rhs alternative strings] }.
// Alternatives continue on lines beginning with '|' and are also
// separated inline by '|'.
function parseGrammar(path) {
  const productions = new Map();
  let current = null;
  for (const raw of readFileSync(path, 'utf8').split(/\r?\n/)) {
    const line = stripComment(raw).trimEnd();
    if (!line.trim()) continue;
    const head = line.match(/^<([^>]+)>\s*::=\s*(.*)$/);
    if (head) {
      current = head[1].trim();
      if (!productions.has(current)) productions.set(current, []);
      for (const alt of splitAlternatives(head[2])) productions.get(current).push(alt);
      continue;
    }
    const alt = line.match(/^\s*\|\s*(.*)$/);
    if (alt && current) {
      for (const a of splitAlternatives(alt[1])) productions.get(current).push(a);
    }
  }
  return productions;
}

// First keyword candidate in one alternative, scanning left to right.
// Returns { keyword, rest } or null when no candidate exists.
function firstKeyword(alt) {
  const tokens = alt.match(/'[^']*'|<[^>]+>|\S+/g) ?? [];
  for (let i = 0; i < tokens.length; i++) {
    const tok = tokens[i];
    // A leading nonterminal means the statement delegates to another
    // production (e.g. <File_Dialog_Statement> ::= <FileDialogStrStatement>).
    if (tok.startsWith('<')) {
      if (i === 0) return { nonterminal: tok.slice(1, -1) };
      continue;
    }
    const bare = tok.startsWith("'") && tok.endsWith("'") ? tok.slice(1, -1) : tok;
    if (!KEYWORD_RE.test(bare)) continue;
    if (TERMINALS.has(bare.toLowerCase())) continue;
    return { keyword: bare.toUpperCase() };
  }
  return null;
}

// All leading keywords of one production (over its alternatives).
// `CANCEL X` is joined into one name.
function commandNames(productions, lhs, depth = 0) {
  if (depth > 4) return [];
  const names = new Set();
  for (const alt of productions.get(lhs) ?? []) {
    // A single alternative may chain: split 'A | B' already handled by the
    // parser; here handle 'CANCEL SELECT'-style two-keyword leads.
    const words = [];
    const tokens = alt.match(/'[^']*'|<[^>]+>|\S+/g) ?? [];
    for (const tok of tokens) {
      if (tok.startsWith('<')) break;
      const bare = tok.startsWith("'") && tok.endsWith("'") ? tok.slice(1, -1) : tok;
      if (!KEYWORD_RE.test(bare)) break;
      if (TERMINALS.has(bare.toLowerCase())) break;
      words.push(bare.toUpperCase());
      if (words.length === 2) break;
    }
    if (words.length === 2 && words[0] === 'CANCEL') {
      names.add(words.join(' '));
      continue;
    }
    const first = firstKeyword(alt);
    if (first?.keyword) {
      names.add(first.keyword);
    } else if (first?.nonterminal) {
      for (const n of commandNames(productions, first.nonterminal, depth + 1)) {
        names.add(n);
      }
    }
  }
  return [...names];
}

// ---- gather the required names -------------------------------------------

const requiredCommands = new Map(); // name -> grammar
const allFunctions = new Set();
const requiredFieldTypes = new Set();

for (const g of GRAMMARS) {
  const productions = parseGrammar(g.path);
  const statementNonterminals = new Set();
  for (const ep of g.entryPoints) {
    for (const alt of productions.get(ep) ?? []) {
      for (const m of alt.matchAll(/<([^>]+)>/g)) statementNonterminals.add(m[1]);
    }
  }
  for (const nt of statementNonterminals) {
    if (NON_COMMAND.has(nt)) continue;
    let names = commandNames(productions, nt);
    if (names.length === 0 && NAME_OVERRIDES.has(nt)) names = [NAME_OVERRIDES.get(nt)];
    if (names.length === 0) {
      fail(`${g.name} grammar: statement production <${nt}> yields no command name (add to NAME_OVERRIDES or NON_COMMAND)`);
      continue;
    }
    for (const n of names) {
      if (!requiredCommands.has(n)) requiredCommands.set(n, g.name);
    }
  }
  // Function-name terminals inside <*FuncName*> productions.
  for (const [lhs, alts] of productions) {
    if (!/funcname/i.test(lhs)) continue;
    for (const alt of alts) {
      for (const m of alt.matchAll(/[A-Za-z][A-Za-z0-9_-]*/g)) {
        const w = m[0].toUpperCase();
        if (!TERMINALS.has(w.toLowerCase())) allFunctions.add(w);
      }
    }
  }
}

for (const path of FUNCTION_DISPATCH) {
  for (const m of readFileSync(path, 'utf8').matchAll(/case\s+"([A-Z0-9_]+)":/g)) {
    allFunctions.add(m[1]);
  }
}

for (const m of readFileSync(FIELD_TYPE_ENUM, 'utf8').matchAll(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*\d+\s*,/gm)) {
  requiredFieldTypes.add(m[1]);
}

// ---- validate the register ------------------------------------------------

const register = JSON.parse(readFileSync(REGISTER, 'utf8'));
const MAX_VERSION = [1, 6];

function checkEntries(section, label, fullShape = true) {
  const seen = new Map();
  for (const e of register[section] ?? []) {
    if (typeof e.name !== 'string' || !e.name) fail(`${label}: entry without a name`);
    const name = String(e.name).toUpperCase();
    if (seen.has(name)) fail(`${label}: "${name}" appears ${seen.get(name) + 1} times (must appear exactly once)`);
    seen.set(name, (seen.get(name) ?? 0) + 1);
    if (!['supported', 'planned', 'not_planned'].includes(e.status)) {
      fail(`${label} "${name}": status "${e.status}" is not supported|planned|not_planned`);
    }
    if (e.status === 'not_planned') {
      if (e.version !== null) fail(`${label} "${name}": not_planned but version is ${JSON.stringify(e.version)} (must be null)`);
      if (!e.reason) fail(`${label} "${name}": not_planned requires a reason`);
    } else {
      if (typeof e.version !== 'string' || !/^\d+\.\d+$/.test(e.version)) {
        fail(`${label} "${name}": version ${JSON.stringify(e.version)} is not an "X.Y" string`);
      } else {
        const [maj, min] = e.version.split('.').map(Number);
        if (maj > MAX_VERSION[0] || (maj === MAX_VERSION[0] && min > MAX_VERSION[1])) {
          fail(`${label} "${name}": version ${e.version} > 1.6 without not_planned`);
        }
      }
    }
    if (fullShape) {
      if (typeof e.affects_data !== 'boolean') fail(`${label} "${name}": affects_data must be boolean`);
      if (!Array.isArray(e.options)) fail(`${label} "${name}": options must be an array`);
    } else if (typeof e.epiora_type !== 'string' || !e.epiora_type) {
      fail(`${label} "${name}": epiora_type must be a non-empty string`);
    }
  }
  return seen;
}

const registeredCommands = checkEntries('commands', 'command');
const registeredFunctions = checkEntries('functions', 'function');
const registeredFieldTypes = checkEntries('field_types', 'field_type', false);

for (const [name] of requiredCommands) {
  if (!registeredCommands.has(name)) fail(`commands: grammar command "${name}" missing from register.json`);
}
for (const name of allFunctions) {
  if (!registeredFunctions.has(name)) fail(`functions: "${name}" missing from register.json`);
}
for (const name of requiredFieldTypes) {
  if (!registeredFieldTypes.has(name.toUpperCase())) fail(`field_types: "${name}" missing from register.json`);
}

// Modules: fixed module set; version rule applies to every entry.
const MODULES = new Set(['Enter', 'MakeView', 'Dashboard', 'Map', 'StatCalc', 'AnthStat']);
for (const e of register.modules ?? []) {
  if (!MODULES.has(e.module)) fail(`modules: "${e.module}" is not one of ${[...MODULES].join(', ')}`);
  if (e.status === 'not_planned') {
    if (!e.reason) fail(`modules ${e.module} "${e.feature}": not_planned requires a reason`);
  } else {
    if (typeof e.version !== 'string' || !/^\d+\.\d+$/.test(e.version)) {
      fail(`modules ${e.module} "${e.feature}": version ${JSON.stringify(e.version)} is not an "X.Y" string`);
    } else {
      const [maj, min] = e.version.split('.').map(Number);
      if (maj > MAX_VERSION[0] || (maj === MAX_VERSION[0] && min > MAX_VERSION[1])) {
        fail(`modules ${e.module} "${e.feature}": version ${e.version} > 1.6`);
      }
    }
  }
}

// ---- report ---------------------------------------------------------------

if (failures.length) {
  console.log(`register-check: ${failures.length} violation(s)`);
  for (const f of failures) console.log(`  FAIL ${f}`);
  process.exit(1);
}
console.log(
  `register-check: OK — ${registeredCommands.size} commands ` +
  `(${requiredCommands.size} required by the grammars), ` +
  `${registeredFunctions.size} functions (${allFunctions.size} required), ` +
  `${registeredFieldTypes.size} field types (${requiredFieldTypes.size} required), ` +
  `${(register.modules ?? []).length} module features`,
);
