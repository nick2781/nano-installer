<#
    Renders a run report as one self-contained HTML page.

    The scripts that run a suite keep two files per run: the plain text report,
    which is what a log, a diff or a grep reads, and this page, which is what a
    person reads. The page carries a verdict, the run's fields, a figure per
    outcome, every case that failed first, one row per target, one row per
    behaviour the coverage document promises with what this run ran for it, one
    row per case with what that case holds and how it ended, the pages a capture
    photographed, and the suite's own output with its result lines coloured --
    so a failure is visible without reading any of it. The tables tell one
    story: a target's row points at the cases that ran in it, and a failing case
    is listed both at the top and in place.
    Everything is inline, images included: no network request, no script, no
    font file, so the page works from a download folder, an artifact zip or a
    share.

    The words the page says come from report_text.ps1 in the language the run
    names, so the same page reads in either of the project's two languages.

    The style follows the project's web language: solid surfaces, hairline
    borders, one blue accent, system fonts, and dark mode from the reader's
    system preference.
#>

. (Join-Path $PSScriptRoot "report_text.ps1")

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

# The class an outcome wears, in the cases table, the behaviour card and the
# output. An outcome that is not an outcome -- a behaviour no case ran for --
# is dimmed rather than coloured like a pass.
function Get-ReportToneClass {
    param([string]$Result)

    if ($Result -eq "FAILED") { return "t-bad" }
    if ($Result -eq "ignored") { return "t-skip" }
    if (-not $Result) { return "t-missing" }
    return "t-ok"
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
        [array]$Cases = @(),
        [array]$Coverage = @(),
        [array]$Uncovered = @(),
        [string]$CoverageIntro = "",
        [array]$Images = @(),
        [array]$Sections = @(),
        [array]$Notes = @(),
        [string]$Language = "en-US",
        [hashtable]$Text = $null,
        [array]$Guide = @()
    )

    if ($null -eq $Text) {
        $Text = Get-ReportText -Language $Language
    }

    # Every word the page says is read here once, so a language is a data file
    # rather than a search through this script.
    $runHeading = Get-ReportPhrase -Text $Text -Key "run.heading"
    $failuresHeading = Get-ReportPhrase -Text $Text -Key "failures.heading"
    $casesHeading = Get-ReportPhrase -Text $Text -Key "cases.heading"
    $snapshotsHeading = Get-ReportPhrase -Text $Text -Key "snapshots.heading"
    $expectationHeading = Get-ReportPhrase -Text $Text -Key "snapshots.expectation"
    $outputSummary = Get-ReportPhrase -Text $Text -Key "output.summary"
    $guideHeading = Get-ReportPhrase -Text $Text -Key "guide.heading"
    $caseColumn = Get-ReportPhrase -Text $Text -Key "cases.column.case"
    $resultColumn = Get-ReportPhrase -Text $Text -Key "cases.column.result"
    $whatColumn = Get-ReportPhrase -Text $Text -Key "cases.column.what"
    $targetColumn = Get-ReportPhrase -Text $Text -Key "cases.column.target"
    $coverageHeading = Get-ReportPhrase -Text $Text -Key "coverage.heading"
    $coverageBehaviourColumn = Get-ReportPhrase -Text $Text -Key "coverage.column.behaviour"
    $coverageCasesColumn = Get-ReportPhrase -Text $Text -Key "coverage.column.cases"

    # The verdict is said in the report's language, while the pill keeps the
    # machine's own word for the style that colours it.
    $failed = $Verdict -ne "passed"
    $verdictKey = "verdict.passed"
    if ($failed) { $verdictKey = "verdict.failed" }
    $verdictLabel = Get-ReportPhrase -Text $Text -Key $verdictKey
    $html = New-Object System.Collections.Generic.List[string]
    $html.Add("<!doctype html>")
    $html.Add("<html lang=""$Language"">")
    $html.Add("<head>")
    $html.Add('<meta charset="utf-8">')
    $html.Add('<meta name="viewport" content="width=device-width, initial-scale=1">')
    $html.Add("<title>$(ConvertTo-ReportHtml "$Eyebrow $Title - $verdictLabel")</title>")
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
code, pre, kbd, summary, .mono { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
main { max-width: 60rem; margin: 0 auto; padding: 32px 16px 64px; }
h1 { font-size: 30px; font-weight: 600; letter-spacing: -0.025em; margin: 0 0 6px; }
h2 { font-size: 16px; font-weight: 600; letter-spacing: -0.025em; margin: 0 0 14px; }
h3 { font-size: 13px; font-weight: 600; margin: 22px 0 8px; color: var(--fg); }
h3:first-child { margin-top: 0; }
.count { font-weight: 400; color: var(--faint); }
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
th { text-align: left; font-size: 12px; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase; color: var(--faint); padding: 8px 10px; border-bottom: 1px solid var(--border); }
th.num, td.num { text-align: right; font-variant-numeric: tabular-nums; }
td { padding: 8px 10px; border-bottom: 1px solid var(--border); font-size: 13px; vertical-align: top; }
tbody tr:last-child td { border-bottom: 0; }
.t-ok { color: var(--ok); font-weight: 600; }
.t-bad { color: var(--bad); font-weight: 600; }
.t-skip { color: var(--warn); font-weight: 600; }
tr.row-bad td { background: color-mix(in srgb, var(--bad) 6%, transparent); }
tr.row-bad td:first-child { box-shadow: inset 2px 0 0 var(--bad); }
.scroll-x { overflow-x: auto; scrollbar-width: thin; }
.scroll-y { max-height: 40rem; overflow-y: auto; scrollbar-width: thin; }
.case-name { font-size: 12.5px; }
.case-module, .sub { display: block; color: var(--faint); font-size: 11px; }
.case-what { color: var(--muted); }
.case-what p { margin: 0; }
.case-note { display: block; color: var(--faint); font-size: 11.5px; font-weight: 400; }
.protects { margin-top: 6px; display: flex; flex-wrap: wrap; gap: 6px; }
.chip { border: 1px solid var(--border); border-radius: 9999px; padding: 1px 8px; font-size: 11px; color: var(--faint); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
.intro { margin: 0 0 16px; color: var(--muted); font-size: 13px; }
.t-missing { color: var(--faint); font-weight: 600; }
.cov-name { font-size: 12.5px; overflow-wrap: anywhere; }
.cov-cases { font-size: 12px; }
.cov-case { display: inline-block; margin: 0 12px 4px 0; }
.cov-case .mono { font-size: 12px; }
.cov-case .t-ok, .cov-case .t-bad, .cov-case .t-skip, .cov-case .t-missing { font-size: 11px; }
details { border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
summary { cursor: pointer; padding: 10px 12px; font-size: 12.5px; color: var(--muted); background: var(--code); }
details[open] summary { border-bottom: 1px solid var(--border); }
pre.log { margin: 0; padding: 14px 16px; max-height: 32rem; overflow: auto; font-size: 12.5px; line-height: 1.7; background: var(--surface); scrollbar-width: thin; }
pre.log span { display: block; white-space: pre-wrap; overflow-wrap: anywhere; }
pre.expectation { margin: 0; padding: 12px 14px; white-space: pre-wrap; font-size: 12px; line-height: 1.65; color: var(--muted); }
.l-ok { color: var(--ok); }
.l-good { color: var(--ok); font-weight: 600; }
.l-fail { color: var(--bad); font-weight: 600; }
.l-skip { color: var(--warn); }
.l-warn { color: var(--warn); }
.l-meta { color: var(--faint); }
.snapshots { display: grid; gap: 24px; }
figure { margin: 0; }
figure img { display: block; width: 100%; height: auto; border: 1px solid var(--border); border-radius: 8px; background: var(--code); }
figcaption { margin-top: 10px; font-size: 12.5px; color: var(--muted); }
figcaption .file { color: var(--faint); }
ul.checks { margin: 8px 0 10px; padding-left: 18px; }
ul.checks li { margin-bottom: 2px; }
ul.guide { margin: 0; padding-left: 18px; color: var(--muted); }
ul.guide li { margin-bottom: 6px; }
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
    $html.Add("<span class=""pill $pill"">$(ConvertTo-ReportHtml $verdictLabel)</span>")
    $html.Add("</header>")

    if ($Guide.Count -gt 0) {
        $html.Add("<section class=""card"">")
        $html.Add("<h2>$(ConvertTo-ReportHtml $guideHeading)</h2>")
        $html.Add("<ul class=""guide"">")
        foreach ($item in $Guide) {
            $html.Add("<li>$(ConvertTo-ReportHtml $item)</li>")
        }
        $html.Add("</ul>")
        $html.Add("</section>")
    }

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
        $html.Add("<h2>$(ConvertTo-ReportHtml $runHeading)</h2>")
        $html.Add("<dl>")
        foreach ($field in $Fields) {
            $html.Add("<div><dt>$(ConvertTo-ReportHtml $field.Label)</dt><dd>$(ConvertTo-ReportHtml $field.Value)</dd></div>")
        }
        $html.Add("</dl>")
        $html.Add("</section>")
    }

    # A failed case is what a reader looks for first, so every case that failed is
    # listed before the tables, each one linked to its own row in the cases table
    # below where its pattern sits among the cases that passed.
    $failures = New-Object System.Collections.Generic.List[object]
    $caseIndex = 0
    foreach ($group in $Cases) {
        $rowIndex = 0
        foreach ($case in $group.Rows) {
            if ($case.Result -eq "FAILED") {
                $failures.Add(@{
                    Anchor = "case-$caseIndex-$rowIndex"
                    Name   = $case.Name
                    Module = $case.Module
                    Target = $group.Target
                    What   = $case.What
                })
            }
            $rowIndex++
        }
        $caseIndex++
    }
    if ($failures.Count -gt 0) {
        $html.Add("<section class=""card"">")
        $failuresCount = Get-ReportPhrase -Text $Text -Key "failures.count" -Values @($failures.Count)
        $html.Add("<h2>$(ConvertTo-ReportHtml $failuresHeading) <span class=""count"">$(ConvertTo-ReportHtml $failuresCount)</span></h2>")
        $html.Add("<div class=""scroll-x"">")
        $html.Add("<table>")
        $html.Add("<thead><tr><th scope=""col"">$(ConvertTo-ReportHtml $caseColumn)</th><th scope=""col"">$(ConvertTo-ReportHtml $targetColumn)</th><th scope=""col"">$(ConvertTo-ReportHtml $whatColumn)</th></tr></thead>")
        $html.Add("<tbody>")
        foreach ($failure in $failures) {
            $html.Add("<tr>")
            $html.Add("<td class=""case-name mono""><a href=""#$($failure.Anchor)"">$(ConvertTo-ReportHtml $failure.Name)</a>")
            if ($failure.Module) {
                $html.Add("<span class=""case-module"">$(ConvertTo-ReportHtml $failure.Module)</span>")
            }
            $html.Add("</td>")
            $html.Add("<td class=""case-what"">$(ConvertTo-ReportHtml $failure.Target)</td>")
            $html.Add("<td class=""case-what"">$(ConvertTo-ReportHtml $failure.What)</td>")
            $html.Add("</tr>")
        }
        $html.Add("</tbody>")
        $html.Add("</table>")
        $html.Add("</div>")
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
        $rowIds = @()
        if ($table.ContainsKey("RowIds")) { $rowIds = @($table.RowIds) }
        $rowIndex = 0
        foreach ($row in $table.Rows) {
            $rowAttribute = ""
            if ($rowIndex -lt $rowIds.Count -and $rowIds[$rowIndex]) {
                $rowAttribute = " id=""$($rowIds[$rowIndex])"""
            }
            $html.Add("<tr$rowAttribute>")
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
                $cellText = ConvertTo-ReportHtml $cell.Text
                if ($cell.ContainsKey("Link")) { $cellText = "<a href=""$($cell.Link)"">$cellText</a>" }
                if ($cell.ContainsKey("Sub")) { $cellText = "$cellText<span class=""sub"">$(ConvertTo-ReportHtml $cell.Sub)</span>" }
                $html.Add("<td$attribute>$cellText</td>")
                $index++
            }
            $html.Add("</tr>")
            $rowIndex++
        }
        $html.Add("</tbody>")
        $html.Add("</table>")
        $html.Add("</div>")
        $html.Add("</section>")
    }

    # What the run did with each behaviour the coverage document promises: the
    # document names the cases that hold a behaviour, this run says how those
    # cases ended, and a behaviour none of whose cases ran is named as such. The
    # part of the document that admits no case covers a thing closes the card,
    # where whoever looks for it finds it rather than reading it as a promise.
    if ($Coverage.Count -gt 0 -or $Uncovered.Count -gt 0) {
        $coverageTotal = 0
        $coverageMissing = 0
        foreach ($row in $Coverage) {
            $coverageTotal++
            if (-not $row.Verdict) { $coverageMissing++ }
        }
        $coverageGroups = New-Object System.Collections.Generic.List[object]
        $currentGroup = $null
        foreach ($row in $Coverage) {
            if ($null -eq $currentGroup -or $currentGroup.Section -ne $row.Section) {
                $currentGroup = @{ Section = $row.Section; Rows = (New-Object System.Collections.Generic.List[object]) }
                $coverageGroups.Add($currentGroup)
            }
            $currentGroup.Rows.Add($row)
        }

        $html.Add("<section class=""card"">")
        $coverageCount = Get-ReportPhrase -Text $Text -Key "coverage.summary" -Values @($coverageTotal)
        if ($coverageMissing -gt 0) {
            $separator = Get-ReportPhrase -Text $Text -Key "counts.separator"
            $missingText = Get-ReportPhrase -Text $Text -Key "coverage.missing" -Values @($coverageMissing)
            $coverageCount = "$coverageCount$separator$missingText"
        }
        $html.Add("<h2>$(ConvertTo-ReportHtml $coverageHeading) <span class=""count"">$(ConvertTo-ReportHtml $coverageCount)</span></h2>")
        if ($CoverageIntro) {
            $html.Add("<p class=""intro"">$(ConvertTo-ReportHtml $CoverageIntro)</p>")
        }
        foreach ($group in $coverageGroups) {
            $groupCount = Get-ReportPhrase -Text $Text -Key "coverage.summary" -Values @($group.Rows.Count)
            $html.Add("<h3>$(ConvertTo-ReportHtml $group.Section) <span class=""count"">$(ConvertTo-ReportHtml $groupCount)</span></h3>")
            $html.Add("<div class=""scroll-x"">")
            $html.Add("<table>")
            $html.Add("<thead><tr><th scope=""col"">$(ConvertTo-ReportHtml $coverageBehaviourColumn)</th><th scope=""col"">$(ConvertTo-ReportHtml $resultColumn)</th><th scope=""col"">$(ConvertTo-ReportHtml $coverageCasesColumn)</th></tr></thead>")
            $html.Add("<tbody>")
            foreach ($row in $group.Rows) {
                $html.Add("<tr>")
                $html.Add("<td class=""cov-name mono"">$(ConvertTo-ReportHtml $row.Behaviour)</td>")
                $html.Add("<td class=""$(Get-ReportToneClass $row.Verdict)"">$(ConvertTo-ReportHtml (Get-ReportResultLabel -Text $Text -Result $row.Verdict))</td>")
                $html.Add("<td class=""cov-cases"">")
                foreach ($case in $row.Cases) {
                    $html.Add("<span class=""cov-case""><span class=""mono"">$(ConvertTo-ReportHtml $case.Name)</span> <span class=""$(Get-ReportToneClass $case.Result)"">$(ConvertTo-ReportHtml (Get-ReportResultLabel -Text $Text -Result $case.Result))</span></span>")
                }
                $html.Add("</td>")
                $html.Add("</tr>")
            }
            $html.Add("</tbody>")
            $html.Add("</table>")
            $html.Add("</div>")
        }
        foreach ($group in $Uncovered) {
            $html.Add("<h3>$(ConvertTo-ReportHtml $group.Section)</h3>")
            $html.Add("<ul class=""notes"">")
            foreach ($line in $group.Lines) {
                $html.Add("<li>$(ConvertTo-ReportHtml $line)</li>")
            }
            $html.Add("</ul>")
        }
        $html.Add("</section>")
    }

    # Every case the run reported, with what it holds: the doc comment beside it
    # in the sources, the behaviours and settings that break with it, and its
    # outcome in this run.
    if ($Cases.Count -gt 0) {
        $caseTotal = 0
        foreach ($group in $Cases) { $caseTotal += $group.Rows.Count }
        $caseSummary = Get-ReportPhrase -Text $Text -Key "cases.summary" -Values @($caseTotal, $Cases.Count)
        $html.Add("<section class=""card"">")
        $html.Add("<h2>$(ConvertTo-ReportHtml $casesHeading) <span class=""count"">$(ConvertTo-ReportHtml $caseSummary)</span></h2>")
        $html.Add("<div class=""scroll-y"">")
        $caseIndex = 0
        foreach ($group in $Cases) {
            $html.Add("<h3 id=""cases-$caseIndex"">$(ConvertTo-ReportHtml $group.Target) <span class=""count"">$(ConvertTo-ReportHtml $group.Counts)</span></h3>")
            $html.Add("<table>")
            $html.Add("<thead><tr><th scope=""col"">$(ConvertTo-ReportHtml $caseColumn)</th><th scope=""col"">$(ConvertTo-ReportHtml $resultColumn)</th><th scope=""col"">$(ConvertTo-ReportHtml $whatColumn)</th></tr></thead>")
            $html.Add("<tbody>")
            $rowIndex = 0
            foreach ($case in $group.Rows) {
                $rowAttribute = " id=""case-$caseIndex-$rowIndex"""
                if ($case.Result -eq "FAILED") { $rowAttribute = "$rowAttribute class=""row-bad""" }
                $html.Add("<tr$rowAttribute>")
                $html.Add("<td class=""case-name mono"">$(ConvertTo-ReportHtml $case.Name)")
                if ($case.Module) {
                    $html.Add("<span class=""case-module"">$(ConvertTo-ReportHtml $case.Module)</span>")
                }
                $html.Add("</td>")
                $html.Add("<td class=""$(Get-ReportToneClass $case.Result)"">$(ConvertTo-ReportHtml (Get-ReportResultLabel -Text $Text -Result $case.Result))")
                if ($case.Note) {
                    $html.Add("<span class=""case-note"">$(ConvertTo-ReportHtml $case.Note)</span>")
                }
                $html.Add("</td>")
                $html.Add("<td class=""case-what"">")
                $html.Add("<p>$(ConvertTo-ReportHtml $case.What)</p>")
                if ($case.Protects.Count -gt 0) {
                    $html.Add("<p class=""protects"">")
                    foreach ($protect in $case.Protects) {
                        $html.Add("<span class=""chip"">$(ConvertTo-ReportHtml $protect)</span>")
                    }
                    $html.Add("</p>")
                }
                $html.Add("</td>")
                $html.Add("</tr>")
                $rowIndex++
            }
            $html.Add("</tbody>")
            $html.Add("</table>")
            $caseIndex++
        }
        $html.Add("</div>")
        $html.Add("</section>")
    }

    if ($Images.Count -gt 0) {
        $html.Add("<section class=""card"">")
        $html.Add("<h2>$(ConvertTo-ReportHtml $snapshotsHeading)</h2>")
        $html.Add("<div class=""snapshots"">")
        foreach ($image in $Images) {
            $html.Add("<figure>")
            $html.Add("<img src=""$($image.Source)"" alt=""$(ConvertTo-ReportHtml $image.Alt)"">")
            $html.Add("<figcaption>")
            $html.Add("<span class=""file mono"">$(ConvertTo-ReportHtml $image.File)</span> - $(ConvertTo-ReportHtml $image.Caption)")
            if ($image.Asserts.Count -gt 0) {
                $html.Add("<ul class=""checks"">")
                foreach ($assert in $image.Asserts) {
                    $html.Add("<li>$(ConvertTo-ReportHtml $assert)</li>")
                }
                $html.Add("</ul>")
            }
            if ($image.Expectation) {
                $html.Add("<details><summary>$(ConvertTo-ReportHtml $expectationHeading)</summary><pre class=""expectation"">$(ConvertTo-ReportHtml $image.Expectation)</pre></details>")
            }
            $html.Add("</figcaption>")
            $html.Add("</figure>")
        }
        $html.Add("</div>")
        $html.Add("</section>")
    }

    foreach ($section in $Sections) {
        $html.Add("<section class=""card"">")
        $html.Add("<h2>$(ConvertTo-ReportHtml $section.Heading)</h2>")
        $open = ""
        if ($section.ContainsKey("Open") -and $section.Open) { $open = " open" }
        $summaryText = $section.Summary
        if (-not $summaryText) { $summaryText = $outputSummary }
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
