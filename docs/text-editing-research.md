# Text Editing: Owning Text and Selection

Status: research, captured 2026-10-03. Nothing here has been prototyped yet. The platform facts come from API documentation and how existing editors are built, not from experiments in this repo.

## Question

Can the Rust model be the true source of truth for both the **text** and the **selection** of a text input on every backend? That is the precondition for building a rich editor on top of the platform abstraction.

"True source of truth" means:

- Every mutation is proposed to the model before it lands, and the model can accept, transform or reject it. This covers typing, IME composition, autocorrect, dictation, paste, drag and drop, undo, accessibility and system features such as Writing Tools.
- The native view never shows a state the model has not approved.
- Programmatic changes from the model reach the native view without breaking IME composition, the caret or undo.

## Where the code is today

`widgets/text_input.rs` defines `TextInput` with `with_on_text_changed` (whole string), `with_on_selection_changed` (after the fact) and `TextCommand::SetText` (whole string). The AppKit backend (`appkit/ui/text_view.rs`) attaches an `NSTextViewDelegate`. The GTK backend (`gtk/ui/text_input.rs`) listens to `changed` and `mark-set` and replaces the buffer on `SetText`.

That is a mirror. The native view edits, we observe it, and we overwrite it. It cannot be the source of truth (see below).

An earlier AppKit `ReactiveTextStorage` (an `NSTextStorage` subclass) was removed in `a482f6f`. `CLAUDE.md` still describes it.

## Four ways to control the text

| | Approach | What we control | Cost |
|---|---|---|---|
| 1 | **Mirror.** Observe native changes, overwrite with `setText`. | Nothing reliably. Breaks IME composition, makes the caret jump, corrupts undo, causes feedback loops. | Low |
| 2 | **Veto.** "Should change" hooks reject or rewrite an edit before it lands. | Text mostly. Rejecting during composition still breaks the IME. | Low |
| 3 | **Own the storage.** The native view reads and writes through our object. | Text fully. Selection depends on the platform. Native caret, handles, menus, spell-check and accessibility keep working. | Medium |
| 4 | **Own the input protocol.** A custom view implements the platform text-input client and does its own layout. | Everything. | High: we draw the caret and selection ourselves, and on some platforms build handles and accessibility too. |

## Per platform

| | Own the text | Own the selection | Composition (marked text / preedit) | Approach 4 |
|---|---|---|---|---|
| **AppKit** | Subclass `NSTextStorage` (approach 3). Works with TextKit 1 and 2. | **Yes, before it applies.** `textView:willChangeSelectionFromCharacterRanges:toCharacterRanges:` can transform the proposed selection. | Marked text lives in the storage, so the model sees it. | `NSTextInputClient` on a custom `NSView`. Mature; Zed and Chromium use it. |
| **UIKit** | Same `NSTextStorage` subclass. | **After the fact only.** `UITextView` reports selection changes once applied; we correct afterwards. | Marked text lives in the storage. Returning `false` from `shouldChangeTextIn` during composition breaks the IME. | `UITextInput` plus `UITextInteraction` (iOS 13+). The system supplies handles, magnifier and edit menu for a custom view. |
| **Android** | `TextView.setEditableFactory` returning our own `Editable`, e.g. a `SpannableStringBuilder` subclass overriding `replace()`. | **Yes.** Selection is stored as spans in the `Editable` (`Selection.SELECTION_START/END`), so owning the `Editable` owns the selection. | The IME writes composing spans straight into the `Editable`. | `onCreateInputConnection` with our own `InputConnection`. Compose's `BasicTextField` (`TextFieldState`, `InputTransformation`) is the reference. |
| **GTK4** | `GtkTextBuffer` storage cannot be swapped. Connect to `insert-text` / `delete-range` before the default handler, stop emission, apply our own edit (approach 2 that behaves like 3). | **After the fact only** (`mark-set`). Marks can be re-set inside the handler. Keyboard movement intent is available by intercepting the `move-cursor` action signal. | **Preedit is not in the buffer** until committed. The cleanest of the four for a model. | `GtkIMContext` (`commit`, `preedit-changed`, `retrieve-surrounding`, `delete-surrounding`) plus `GtkAccessibleText` (4.14+). Small API that suits a model. |

## Constraints on every platform

1. **Edits must be applied synchronously.** Every text system reads back right after an edit (`replaceCharacters`, `replace()`, `insert-text`). The model must accept and apply the edit inside the native callback, not through a channel and a later tick. Here that means the callback mutates the `StoredSignal` synchronously, and the effect pushing model changes back to the view skips changes that came from the view, using a revision number. Otherwise every keystroke echoes back.
2. **Composition is part of the model.** State needs roughly `{ text, selection, composition: Option<Range>, revision }`. A programmatic change during composition must tell the IME:
   - AppKit: `discardMarkedText` or `invalidateCharacterCoordinates`
   - UIKit: `inputDelegate.textWillChange` / `textDidChange`
   - Android: `InputMethodManager.updateSelection` or `restartInput`
   - GTK: `gtk_im_context_reset`

   Skipping this is the usual cause of CJK input desyncing.
3. **Changes are range-based, never whole-string.** `SetText(full)` resets composition and selection everywhere. Commands and callbacks both need the shape `Edit { range, replacement, origin }`.
4. **Undo belongs to the model.** Turn off native undo (`allowsUndo = NO`, the UIKit undo manager, GTK `enable-undo = false`), or the native stack replays edits behind the model's back.
5. **Index units differ.** Apple and Android use UTF-16 code units; GTK uses codepoints for offsets. `ui-core/src/encoding.rs` has the conversions. A rich editor's model will want a rope that answers in both units cheaply.
6. **Android: mind the JNI boundary.** The text system reads characters constantly, so an `Editable` that calls into Rust per character is too slow. Keep a Java-side copy that forwards each `replace()` synchronously to Rust, which approves, transforms or rejects it.
7. **Rich text maps onto native formatting.** Model attribute runs drive `NSTextStorage` attributes (Apple), spans (Android) and `GtkTextTag` (GTK). Inline objects become `NSTextAttachment` or a TextKit 2 view provider, `ReplacementSpan`, and GTK child anchors.

## When selection can only be corrected afterwards

This applies to approach 3 on UIKit and GTK.

**What goes wrong:**

1. **Side effects have already fired.** The IME has been told about the selection (UIKit `selectionDidChange`, GTK IM cursor update). Correcting it sends a second notification, can commit or cancel in-progress composition, and can make autocorrect act on the wrong word.
2. **Corrections fight gestures.** Snapping a selection back while the user drags a handle makes the handle and magnifier jitter. The workaround is to tolerate illegal selections during a drag and normalise when it ends. UIKit does not cleanly report when a handle drag ends.
3. **Correcting re-triggers the callback.** A guard is needed, and normalisation rules must be idempotent so they cannot ping-pong.
4. **We see results, not intent.** Direction can be inferred from old and new selections. Word, line or paragraph granularity cannot. GTK's `move-cursor` recovers keyboard intent; pointer input stays opaque.
5. **A frame of wrong state** appears if the correction goes through a reactive effect on the next tick. Corrections must be synchronous, like edits.

**Which rich-editor features are affected:**

| Feature | Affected? |
|---|---|
| Atomic inline objects (mentions, chips, images) | Avoidable: represent each as one U+FFFC attachment character so the caret cannot enter it. |
| Grapheme, emoji, bidi caret rules | No. Native handles these. |
| Collaborative cursors and highlights | No. They are decorations, not the native selection. |
| Non-editable or hidden regions (e.g. collapsed markdown syntax) | Yes. Needs snap-back correction, with the gesture problem. |
| Block or node selection (a whole table or image) | Yes. Not expressible as one native text range. |
| Multi-cursor editing | Yes, but no native text view supports it (AppKit can show several ranges; typing does not go to all of them). |
| Custom word, line or paragraph selection semantics | Yes. Intent is not visible. |

**Design around it:**

- **The model's selection is the truth; the native selection is a projection.** The model holds a rich logical selection (ranges, node selection, multiple cursors). The native selection is one range that keeps the keyboard, IME and accessibility happy. What the native view cannot express is drawn as decorations. Native-to-model changes are interpreted as intent, not copied. ProseMirror and Lexical do this on the web, where `contenteditable` has the same weakness.
- **Make illegal positions impossible rather than corrected.** Use attachment characters for atomic items, and keep hidden text out of the native storage entirely. Each snap-back designed out removes one of the problems above.
- **Never correct during composition**, and defer during gestures where they can be detected.
- **Approach 4 is cheapest on exactly these two platforms.** On UIKit, `UITextInteraction` changes the selection by calling our `selectedTextRange` setter. With `GtkIMContext` there is no native selection at all.

## What "perfect" costs

A perfect solution means approach 4: we rebuild editing **behaviour**. Every serious rich editor ends up here (ProseMirror, Lexical, Monaco, Google Docs, Zed, Flutter, Compose).

**Stays native even with approach 4:**

- IME and keyboards, via the platform client protocol
- Shaping, layout, bidi and font fallback (CoreText/TextKit, Pango, `StaticLayout`)
- Word, line and grapheme boundaries (`CFStringTokenizer`, Pango log attributes, `BreakIterator`)
- Spell-check services (`NSSpellChecker`, `UITextChecker`, `SpellCheckerSession`; GTK has none, see libspelling)
- Clipboard, drag and drop, and the accessibility bridge (we feed it, we do not build it)

**What we rebuild, per platform:**

| | Caret and selection drawing, hit testing | Movement and keybindings | Handles, magnifier, context menu | Overall cost |
|---|---|---|---|---|
| AppKit | Us | Free: `doCommandBySelector:` delivers intent (`moveWordLeft:`, etc.) | Context menu is simple | Moderate |
| UIKit | Us | `UITextInput` tokenizer | Free via `UITextInteraction` | Moderate |
| GTK | Us | Us, but small | Rarely needed | Low |
| Android | Us | Us | Us (Compose did this) | Highest |

Undo, autoscroll and accessibility text roles are ours everywhere. Undo is ours anyway once the model owns the text.

## Recommendation

The part that must be built regardless is the **editor model**: document schema, transactions, logical selection, undo, later collaboration. No native view provides it. Platform adapters are replaceable projections of that model.

1. **Define the contract in Rust.** `EditorState` (text, logical selection, composition, revision), plus synchronous `propose_edit(Edit) -> EditDecision` and `propose_selection(native_range) -> Selection`. On AppKit and Android `propose_selection` runs before the change applies; on UIKit and GTK it runs as an immediate correction. The editor sees one contract everywhere.
2. **Implement every backend with approach 3 first.** `NSTextStorage` subclass on Apple, custom `Editable` on Android, pre-default signal handlers on GTK. True text ownership with native feel; imperfect only around selection on UIKit and GTK.
3. **Move a backend to approach 4 when a feature needs it** (hidden regions, block selection). GTK and UIKit first, as they are cheapest. Android last, using Compose's `BasicTextField` / `TextFieldState` source as the blueprint.

What has to be right from day one is the contract: model changes are never whole-string sets, and the model's selection is always the truth. The current `TextInput` trait (`on_text_changed(&str)`, `SetText`) should be replaced with that shape before any rich-editor work starts.
