<script>
  import { onMount, onDestroy } from 'svelte';
  import { EditorView, basicSetup } from 'codemirror';
  import { languages } from '@codemirror/language-data';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { EditorState, Compartment } from '@codemirror/state';
  import { LanguageDescription } from '@codemirror/language';

  let {
    value = $bindable(''),
    file = '',
    theme = 'midnight',
    readOnly = false,
    clickableTokens = [],
    onTokenClick = null
  } = $props();

  let editorContainer;
  let view;
  const languageConf = new Compartment();
  const readOnlyConf = new Compartment();
  let clickableTokenSet = $state(new Set());

  function isTokenChar(char) {
    return /[A-Za-z0-9_./:-]/.test(char);
  }

  function readTokenAtPosition(docText, pos) {
    if (!docText || pos == null || pos < 0 || pos > docText.length) return null;

    let idx = pos;
    if (idx >= docText.length) idx = docText.length - 1;
    if (idx < 0) return null;

    if (!isTokenChar(docText[idx]) && idx > 0 && isTokenChar(docText[idx - 1])) {
      idx -= 1;
    }
    if (!isTokenChar(docText[idx])) return null;

    let start = idx;
    let end = idx + 1;
    while (start > 0 && isTokenChar(docText[start - 1])) start -= 1;
    while (end < docText.length && isTokenChar(docText[end])) end += 1;

    return docText.slice(start, end);
  }

  async function updateLanguage(filename) {
    if (!view) return;
    
    const desc = LanguageDescription.matchFilename(languages, filename) || 
                 LanguageDescription.matchLanguageName(languages, filename.split('.').pop() || '');

    if (desc) {
      const lang = await desc.load();
      view.dispatch({
        effects: languageConf.reconfigure(lang)
      });
    } else {
      view.dispatch({
        effects: languageConf.reconfigure([])
      });
    }
  }

  $effect(() => {
    // Handle external value changes (e.g. loading a new file)
    if (view && value !== view.state.doc.toString()) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value }
      });
    }
  });

  $effect(() => {
    // Update language when file prop changes
    if (view && file) {
      updateLanguage(file);
    }
  });

  $effect(() => {
    clickableTokenSet = new Set(
      (clickableTokens || [])
        .filter(token => typeof token === 'string')
        .map(token => token.trim())
        .filter(token => token.length > 0)
    );
  });

  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: readOnlyConf.reconfigure(EditorState.readOnly.of(readOnly))
    });
  });

  onMount(async () => {
    const state = EditorState.create({
      doc: value,
      extensions: [
        basicSetup,
        languageConf.of([]),
        readOnlyConf.of(EditorState.readOnly.of(readOnly)),
        theme === 'midnight' ? oneDark : [],
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            value = update.state.doc.toString();
          }
        }),
        EditorView.domEventHandlers({
          mousedown: (event, editorView) => {
            if (!onTokenClick || clickableTokenSet.size === 0) return false;

            const pos = editorView.posAtCoords({ x: event.clientX, y: event.clientY });
            if (pos == null) return false;

            const token = readTokenAtPosition(editorView.state.doc.toString(), pos);
            if (!token || !clickableTokenSet.has(token)) return false;

            event.preventDefault();
            onTokenClick(token);
            return true;
          }
        }),
        EditorView.theme({
          "&": { height: "100%" },
          ".cm-scroller": { overflow: "auto" }
        })
      ]
    });

    view = new EditorView({
      state,
      parent: editorContainer
    });

    if (file) updateLanguage(file);
  });

  onDestroy(() => {
    if (view) view.destroy();
  });
</script>

<div class="code-editor-wrapper" bind:this={editorContainer}></div>

<style>
  .code-editor-wrapper {
    width: 100%;
    flex: 1; /* Expand to fill remaining space */
    display: flex;
    flex-direction: column;
    border: 1px solid var(--bg-300);
    border-bottom: none; /* Stick to bottom edge */
    background: var(--bg-100);
    min-height: 0;
  }
</style>
