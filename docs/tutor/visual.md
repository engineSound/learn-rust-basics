The learner's code fails, and they asked to *see* what it does. Build one
animated step-through of **their own code** for the app's visualizer, so they
can watch where it goes wrong.

## Exercise: {{title}}

{{lesson}}

## Their current code ({{file}})

```
{{code}}
```

## Result of running the tests just now

{{result}}

## How the visualizer works

The scene has a few labelled regions (columns), such as "Your call",
"Inside describe(status)" and "Console". Each step highlights one line of
`code`, says what happens in plain words, and changes the boxes:

- `set` adds a box (`id`, `in` = region id, `title`, `value`, `tag`, `sub`,
  `state`), or updates an existing box by `id`. A box whose `in` changes slides
  to the new region. Fields you leave as "" keep their previous value.
- `drop` removes boxes by id.
- `arrows` are the arrows shown in this step; they are not carried over, so
  repeat any you want to keep. `from` and `to` are box ids that exist in this step.
- `line` is the 1-based line of `code` to highlight (0 = none). `rule` is the
  general rule behind the step ("" = none). `error` is a red error banner ("" = none).
- Box states: normal, ok, active (what's happening now), info, bad (where it
  goes wrong), moved, faded (done with). Arrow states: ok, accent, muted, bad;
  styles: solid, dashed.

This is how the lesson's own visual looks (style example; don't copy its content):

```json
{{example}}
```

## What to make

1. Pick the first failing test. `code` is a short excerpt: the part of their
   code that runs for that test (keep their lines exactly, comments may go),
   then one last line that makes the call the test makes.
2. 4 to 8 steps. Trace what really happens with the test's input, value by
   value, until the wrong result (or the error) appears. Mark the line where
   it goes wrong with state "bad", and put the expected value next to what
   they got in the last step's `error`.
3. Don't fix their code and don't show the solution. At most, the last
   step's `rule` names the idea to look at again.
4. Regions: 2 to 4, short labels; a "Console" region has kind "output".
   Box ids are short and unique. Plain words; contrast with Java where it helps.
5. `check` lets the app confirm your trace on their real code:
   `call` is {{call_rule}}, and `prints` is exactly what that prints. If their
   code doesn't {{compile_word}}, `call` is "" and `prints` is the error
   code or error name the compiler or runtime reports.
