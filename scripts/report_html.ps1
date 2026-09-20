<#
    Renders a run report as one self-contained HTML page.

    The scripts that run a suite keep two files per run: the plain text report,
    which is what a log, a diff or a grep reads, and this page, which is what a
    person reads. The page carries a verdict, the run's fields, a figure per
    outcome, one table per set of measurements, and the suite's own output with
    its result lines coloured, so a failure is visible without reading any of
    it. Everything is inline -- no network request, no script, no font file --
    so the page works from a download folder, an artifact zip or a share.

    The style follows the project's web language: solid surfaces, hairline
    borders, one blue accent, system fonts, and dark mode from the reader's
    system preference.
#>

function ConvertTo-ReportHtml {
    param([string]$Text)

    $escaped = $Text.Replace("&", "&amp;")
    $escaped = $escaped.Replace("<", "&lt;")
    $escaped = $escaped.Replace(">", "&gt;")
    $escaped = $escaped.Replace('"', "&quot;")
    return $escaped
}

# Which colour a line of a suite's output earns. A result line carries the
# verdict, a case line carries its own outcome, and everything the tools say
# around them is context.
function Get-ReportLineClass {
    param([string]$Line)

    if ($Line -match "^test result: FAILED" -or $Line -match "^error" -or $Line -match " FAILED$") {
        return "l-fail"
    }
    if ($Line -match "^test result: ok") {
        return "l-good"
    }
    if ($Line -match " FAILED" -or $Line -match "^failures:") {
        return "l-fail"
    }
    if ($Line -match " ok$") {
        return "l-ok"
    }
    if ($Line -match "ignored" -or $Line -match "^skipping") {
        return "l-skip"
    }
    if ($Line -match "^warning") {
        return "l-warn"
    }
    if ($Line -match "^\s*(Running|Compiling|Finished|Downloading|Updating|Blocking|Doc-tests|Documenting|Building)\b") {
        return "l-meta"
    }
    return ""
}

function Write-ReportHtml {
    param(
        [string]$Path,
        [string]$Title,
        [string]$Eyebrow = "nano-installer",
        [string]$Subtitle = "",
        [string]$Verdict = "passed",
        [array]$Fields = @(),
        [array]$Stats = @(),
        [array]$Tables = @(),
        [array]$Sections = @(),
        [array]$Notes = @()
    )

    $failed = $Verdict -ne "passed"
    $html = New-Object System.Collections.Generic.List[string]
    $html.Add("<!doctype html>")
    $html.Add('<html lang="en">')
    $html.Add("<head>")
    $html.Add('<meta charset="utf-8">')
    $html.Add('<meta name="viewport" content="width=device-width, initial-scale=1">')
    $html.Add("<title>$(ConvertTo-ReportHtml "$Eyebrow $Title - $Verdict")</title>")
    $html.Add('<style>')
    $css = @'
:root {
  color-scheme: light;
  --bg: #ffffff; --surface: #ffffff; --code: #f9fafb;
  --border: #e5e7eb; --fg: #111827; --muted: #4b5563; --faint: #6b7280;
  --brand: #0b57d0; --ok: #1a7f37; --bad: #cf222e; --warn: #9a6700;
  --radius: 12px;
}
@media (prefers-color-scheme: dark) {
  :root {
    color-scheme: dark;
    --bg: #000000; --surface: #0d0d0d; --code: #0d0d0d;
    --border: #1f1f1f; --fg: #f3f4f6; --muted: #9ca3af; --faint: #6b7280;
    --brand: #8ab4f8; --ok: #3fb950; --bad: #f85149; --warn: #d29922;
  }
}
* { box-sizing: border-box; }
body {
  margin: 0; background: var(--bg); color: var(--fg);
  font: 14px/1.5 ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif;
  -webkit-font-smoothing: antialiased;
}
code, pre, kbd, summary { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
main { max-width: 60rem; margin: 0 auto; padding: 32px 16px 64px; }
h1 { font-size: 30px; font-weight: 600; letter-spacing: -0.025em; margin: 0 0 6px; }
h2 { font-size: 16px; font-weight: 600; letter-spacing: -0.025em; margin: 0 0 14px; }
a { color: var(--brand); }
.eyebrow { margin: 0 0 8px; font-size: 11px; font-weight: 600; letter-spacing: 0.12em; text-transform: uppercase; color: var(--brand); }
.head { display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-start; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 20px; margin-bottom: 24px; }
.subtitle { margin: 0; color: var(--faint); font-size: 13px; }
.pill { border-radius: 9999px; padding: 5px 14px; font-size: 12px; font-weight: 500; border: 1px solid var(--border); color: var(--muted); }
.pill.passed { color: var(--ok); border-color: color-mix(in srgb, var(--ok) 40%, transparent); background: color-mix(in srgb, var(--ok) 10%, transparent); }
.pill.failed { color: var(--bad); border-color: color-mix(in srgb, var(--bad) 40%, transparent); background: color-mix(in srgb, var(--bad) 10%, transparent); }
.stats { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; margin: 0 0 24px; }
@media (min-width: 640px) { .stats { grid-template-columns: repeat(4, 1fr); } }
.stat { border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); padding: 14px 16px; }
.stat b { display: block; font-size: 24px; font-weight: 600; letter-spacing: -0.025em; }
.stat span { font-size: 12px; color: var(--faint); }
.tone-ok b { color: var(--ok); }
.tone-bad b { color: var(--bad); }
.tone-muted b { color: var(--faint); }
.card { border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); padding: 20px 24px; margin: 0 0 20px; }
dl { margin: 0; display: grid; gap: 10px; }
dl div { display: grid; grid-template-columns: 7.5rem 1fr; gap: 12px; align-items: baseline; }
dt { color: var(--faint); font-size: 12px; letter-spacing: 0.06em; text-transform: uppercase; }
dd { margin: 0; overflow-wrap: anywhere; }
table { width: 100%; border-collapse: collapse; }
.scroll-x { overflow-x: auto; scrollbar-width: thin; }
th { text-align: left; font-size: 12px; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase; color: var(--faint); padding: 8px 10px; border-bottom: 1px solid var(--border); }
th.num, td.num { text-align: right; font-variant-numeric: tabular-nums; }
td { padding: 8px 10px; border-bottom: 1px solid var(--border); font-size: 13px; }
tbody tr:last-child td { border-bottom: 0; }
.t-ok { color: var(--ok); font-weight: 600; }
.t-bad { color: var(--bad); font-weight: 600; }
details { border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
summary { cursor: pointer; padding: 10px 12px; font-size: 12.5px; color: var(--muted); background: var(--code); }
details[open] summary { border-bottom: 1px solid var(--border); }
pre.log { margin: 0; padding: 14px 16px; max-height: 32rem; overflow: auto; font-size: 12.5px; line-height: 1.7; background: var(--surface); scrollbar-width: thin; }
pre.log span { display: block; white-space: pre-wrap; overflow-wrap: anywhere; }
.l-ok { color: var(--ok); }
.l-good { color: var(--ok); font-weight: 600; }
.l-fail { color: var(--bad); font-weight: 600; }
.l-skip { color: var(--warn); }
.l-warn { color: var(--warn); }
.l-meta { color: var(--faint); }
ul.notes { margin: 0; padding-left: 18px; color: var(--muted); font-size: 13px; }
ul.notes li { margin-bottom: 4px; }
footer { border-top: 1px solid var(--border); margin-top: 24px; padding-top: 16px; color: var(--faint); font-size: 12px; }
footer p { margin: 0 0 6px; }
@media (prefers-reduced-motion: reduce) { * { animation: none !important; transition: none !important; } }
'@
    $html.AddRange([string[]]($css -split "\r?\n"))
    $html.Add('</style>')
    $html.Add("</head>")
    $html.Add("<body>")
    $html.Add("<main>")

    $html.Add("<header class=""head"">")
    $html.Add("<div>")
    $html.Add("<p class=""eyebrow"">$(ConvertTo-ReportHtml $Eyebrow)</p>")
    $html.Add("<h1>$(ConvertTo-ReportHtml $Title)</h1>")
    if ($Subtitle) {
        $html.Add("<p class=""subtitle"">$(ConvertTo-ReportHtml $Subtitle)</p>")
    }
    $html.Add("</div>")
    $pill = "passed"
    if ($failed) { $pill = "failed" }
    $html.Add("<span class=""pill $pill"">$(ConvertTo-ReportHtml $Verdict)</span>")
    $html.Add("</header>")

    if ($Stats.Count -gt 0) {
        $html.Add("<section class=""stats"">")
        foreach ($stat in $Stats) {
            $statClass = "stat"
            if ($stat.ContainsKey("Tone") -and $stat.Tone) { $statClass = "$statClass tone-$($stat.Tone)" }
            $html.Add("<div class=""$statClass""><b>$(ConvertTo-ReportHtml "$($stat.Value)")</b><span>$(ConvertTo-ReportHtml $stat.Label)</span></div>")
        }
        $html.Add("</section>")
    }

    if ($Fields.Count -gt 0) {
        $html.Add("<section class=""card"">")
        $html.Add("<h2>Run</h2>")
        $html.Add("<dl>")
        foreach ($field in $Fields) {
            $html.Add("<div><dt>$(ConvertTo-ReportHtml $field.Label)</dt><dd>$(ConvertTo-ReportHtml $field.Value)</dd></div>")
        }
        $html.Add("</dl>")
        $html.Add("</section>")
    }

    foreach ($table in $Tables) {
        $html.Add("<section class=""card"">")
        $html.Add("<h2>$(ConvertTo-ReportHtml $table.Heading)</h2>")
        $numericFrom = 99
        if ($table.ContainsKey("NumericFrom")) { $numericFrom = $table.NumericFrom }
        $html.Add("<div class=""scroll-x"">")
        $html.Add("<table>")
        $html.Add("<thead><tr>")
        $index = 0
        foreach ($header in $table.Headers) {
            $class = ""
            if ($index -ge $numericFrom) { $class = " class=""num""" }
            $html.Add("<th scope=""col""$class>$(ConvertTo-ReportHtml $header)</th>")
            $index++
        }
        $html.Add("</tr></thead>")
        $html.Add("<tbody>")
        foreach ($row in $table.Rows) {
            $html.Add("<tr>")
            $index = 0
            foreach ($cell in $row) {
                $class = ""
                if ($index -ge $numericFrom) { $class = "num" }
                if ($cell.ContainsKey("Class")) {
                    if ($class) { $class = "$class $($cell.Class)" } else { $class = $cell.Class }
                }
                $attribute = ""
                if ($class) { $attribute = " class=""$class""" }
                if ($cell.ContainsKey("Title")) { $attribute = "$attribute title=""$(ConvertTo-ReportHtml $cell.Title)""" }
                $html.Add("<td$attribute>$(ConvertTo-ReportHtml $cell.Text)</td>")
                $index++
            }
            $html.Add("</tr>")
        }
        $html.Add("</tbody>")
        $html.Add("</table>")
        $html.Add("</div>")
        $html.Add("</section>")
    }

    foreach ($section in $Sections) {
        $html.Add("<section class=""card"">")
        $html.Add("<h2>$(ConvertTo-ReportHtml $section.Heading)</h2>")
        $open = ""
        if ($section.ContainsKey("Open") -and $section.Open) { $open = " open" }
        $summaryText = $section.Summary
        if (-not $summaryText) { $summaryText = "output" }
        $html.Add("<details$open><summary>$(ConvertTo-ReportHtml $summaryText)</summary>")
        $html.Add("<pre class=""log"">")
        foreach ($line in $section.Lines) {
            $class = Get-ReportLineClass $line
            $attribute = ""
            if ($class) { $attribute = " class=""$class""" }
            $html.Add("<span$attribute>$(ConvertTo-ReportHtml $line)</span>")
        }
        $html.Add("</pre>")
        $html.Add("</details>")
        $html.Add("</section>")
    }

    if ($Notes.Count -gt 0) {
        $html.Add("<footer>")
        $html.Add("<ul class=""notes"">")
        foreach ($note in $Notes) {
            $html.Add("<li>$(ConvertTo-ReportHtml $note)</li>")
        }
        $html.Add("</ul>")
        $html.Add("</footer>")
    }

    $html.Add("</main>")
    $html.Add("</body>")
    $html.Add("</html>")

    [System.IO.File]::WriteAllLines($Path, $html, (New-Object System.Text.UTF8Encoding($false)))
}
