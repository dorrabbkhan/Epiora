// Intentional violations for lint_rules_catch_violations — every raw
// element the component-set rule bans (spec §9.2). Ignored by the
// normal eslint run (see eslint.config.js).
export function Violations() {
  return (
    <div>
      <button>raw button</button>
      <input type="text" />
      <select>
        <option>x</option>
      </select>
      <textarea />
      <a href="/x" onClick={() => undefined}>
        click
      </a>
    </div>
  );
}
