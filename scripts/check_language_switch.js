/*
 * Checks the language switch in docs/index.html.
 *
 * The switch is the only script on the site that has to behave, and it decides
 * things a reader would notice at once: which language the site root opens in,
 * which page the switch moves to, and where a page with no twin sends them. It
 * runs in a browser, which a runner does not have, so this loads the shell's own
 * script against a small stand-in for the browser and asks it those questions.
 *
 * It also compares the switch's list of paired pages with the files in the two
 * trees, which scripts/audit_docs_languages.ps1 does from the other side.
 *
 * No dependencies, and nothing to install: run it with node.
 *
 * Usage:
 *   node scripts/check_language_switch.js
 */
'use strict';

const fs = require('fs');
const path = require('path');

const repoRoot = path.join(__dirname, '..');
const shellPath = path.join(repoRoot, 'docs', 'index.html');
const docsDirectory = path.join(repoRoot, 'docs');

const shell = fs.readFileSync(shellPath, 'utf8');
const start = shell.indexOf('<script>');
const end = shell.indexOf('</script>', start);
if (start < 0 || end < 0) {
  throw new Error(shellPath + ' has no inline script to check');
}
const code = shell.slice(start + '<script>'.length, end);

let failures = 0;

function check(name, actual, expected) {
  if (JSON.stringify(actual) === JSON.stringify(expected)) {
    console.log('ok   ' + name);
    return;
  }
  failures += 1;
  console.log('FAIL ' + name + '\n  expected: ' + JSON.stringify(expected) + '\n  actual:   ' + JSON.stringify(actual));
}

// A stand-in for the parts of the browser the script touches: enough to build
// the dropdown, run the hooks, and see where a change would navigate.
function makeDocument() {
  const created = [];

  function element(tag) {
    const node = {
      tagName: tag,
      children: [],
      attributes: {},
      className: '',
      id: '',
      value: '',
      textContent: '',
      parentNode: null,
      appendChild(child) {
        child.parentNode = node;
        node.children.push(child);
        return child;
      },
      insertBefore(child) {
        child.parentNode = node;
        node.children.unshift(child);
        return child;
      },
      setAttribute(name, value) {
        node.attributes[name] = value;
      },
      addEventListener(name, handler) {
        node.handlers = node.handlers || {};
        node.handlers[name] = handler;
      }
    };
    created.push(node);
    return node;
  }

  const body = element('body');
  const documentElement = element('html');
  const searchBox = element('input');
  const document = {
    body,
    documentElement,
    createElement: element,
    getElementById(id) {
      return created.find((node) => node.id === id) || null;
    },
    querySelector(selector) {
      return selector === '.search input' ? searchBox : null;
    }
  };
  return { document, searchBox };
}

// Loads the shell script the way the browser would, then runs the hooks once so
// the dropdown exists, and hands back what a reader's click would do.
function load(language, stored, hash) {
  const { document, searchBox } = makeDocument();
  const replacements = [];
  const window = {
    location: {
      hash,
      pathname: '/nano-installer/',
      search: '',
      replace(url) {
        replacements.push(url);
        window.location.hash = url.slice(url.indexOf('#'));
      }
    },
    setTimeout() {},
    localStorage: {
      getItem() {
        return stored || null;
      },
      setItem() {}
    },
    $docsify: null
  };
  const navigator = { language, languages: [language] };

  new Function('window', 'document', 'navigator', code)(window, document, navigator);

  window.$docsify.plugins[0](
    { afterEach() {}, doneEach(handler) { handler(); } },
    { route: { path: window.location.hash.replace(/^#\/?/, '') } }
  );

  const select = document.getElementById('nano-language');
  return { window, document, searchBox, select, replacements };
}

function switchTo(page, languageId) {
  page.select.value = languageId;
  page.select.handlers.change();
  return page.window.location.hash;
}

// The site root opens in the reader's language, and remembers a choice.
check('root, a Chinese browser', load('zh-CN', '', '').replacements[0], '/nano-installer/#/zh-CN/README.md');
check('root, any other browser', load('fr-FR', '', '#/').replacements[0], '/nano-installer/#/en/README.md');
check('root, a stored choice wins', load('en-US', 'zh-CN', '').replacements[0], '/nano-installer/#/zh-CN/README.md');
check('an address that names a page is left alone', load('zh-CN', '', '#/en/QUICK_START.md').replacements, []);

// The dropdown states the language of the page and moves to the same page.
const quickStart = load('en-US', '', '#/zh-CN/QUICK_START.md');
check('shows the language of the page', quickStart.select.value, 'zh-CN');
check('offers both languages', quickStart.select.children.map((option) => option.textContent), ['中文', 'English']);
check('moves to the same page', switchTo(quickStart, 'en'), '#/en/QUICK_START.md');

// A page one tree carries goes to the same page in the other tree, and a page no
// tree carries goes to that language's landing page.
check('a paired page', switchTo(load('en-US', '', '#/en/TEST_COVERAGE.md'), 'zh-CN'), '#/zh-CN/TEST_COVERAGE.md');
check('a page with no twin', switchTo(load('en-US', '', '#/zh-CN/TEST_CASES.md'), 'en'), '#/en/README.md');
const agentGuide = load('en-US', '', '#/AGENTS.md');
check('a page outside the trees', agentGuide.window.location.hash, '#/AGENTS.md');
check('a page outside the trees, switched', switchTo(agentGuide, 'zh-CN'), '#/zh-CN/README.md');

// Two details a reader notices: the document language, and the search box.
check('the document language follows the page', agentGuide.document.documentElement.lang, 'en');
check('the search box follows the page', agentGuide.searchBox.attributes.placeholder, 'Search');
check('the search box in Chinese', load('zh-CN', '', '#/zh-CN/QUICK_START.md').searchBox.attributes.placeholder, '搜索');

// The switch's list against the trees it switches between.
const listed = code
  .slice(code.indexOf('/* nano-language-pages:start */'), code.indexOf('/* nano-language-pages:end */'))
  .match(/'[^']+\.md'/g)
  .map((entry) => entry.slice(1, -1));
const names = (tree) =>
  fs
    .readdirSync(path.join(docsDirectory, tree))
    .filter((name) => name.endsWith('.md') && !name.startsWith('_'));
const english = names('en');
const chinese = names('zh-CN');
const paired = english.filter((name) => chinese.includes(name));
check('the switch lists the paired pages', listed.slice().sort(), paired.slice().sort());
check('every paired page is in the switch', paired.every((name) => listed.includes(name)), true);

console.log(failures === 0
  ? 'Language switch checked: ' + paired.length + ' paired page(s), ' + listed.length + ' page(s) in the switch'
  : failures + ' check(s) failed');
process.exit(failures === 0 ? 0 : 1);
