<#
    Reads what a report shows about the cases and the pages behind a run.

    A report is worth reading when a row says what the case holds, not only that
    it passed, so both of these read the repository rather than the run:

    - Get-TestCatalog reads every case out of the Rust sources it can find under
      crates/: the doc comment above a #[test] says what the case checks, and
      the covered behaviours come from docs/<language>/TEST_COVERAGE.md, so the
      chips beside a case read in the report's language. What a case checks is
      read from docs/<language>/TEST_CASES.md where that document exists, since
      the doc comments are the sources' own English; a case it does not name
      falls back to the doc comment above it, or to its own name.
    - Get-TestCoverage reads that same document the other way round: every
      behaviour it promises with the cases it names for it, and the part of it
      that admits no case covers a thing. A report that lists a behaviour's
      cases beside their outcome says what was tested, not only what passed.
    - Get-SnapshotGallery reads the manifest capture_setup_snapshots.ps1 writes
      beside its page photographs, so a report can show every page of a real
      setup and what the capture measured on each of them. The manifest names
      each page, and records each check by name rather than in words, so the
      report says both in the language it is read in.
#>

. (Join-Path $PSScriptRoot "report_text.ps1")

# A tool that colours its own status lines writes escape sequences, and a line
# captured from a console can keep the carriage return that ended it. Neither
# belongs in a report, and both get in the way of reading a line: the escape
# sequences in front of cargo's "Running" line stopped this project's own
# reports from grouping the cases under the target that ran them on a build
# agent, where cargo is told to colour its output.
function ConvertTo-ReportLine {
    param([string]$Line)

    $plain = [regex]::Replace($Line, "$([char]27)\[[0-9;?]*[A-Za-z]", "")
    return $plain.TrimEnd([char]13)
}

# A case that has no doc comment is still readable: its name is a sentence
# spelled with underscores.
function ConvertTo-CaseSentence {
    param([string]$Name)

    $sentence = $Name.Replace("_", " ").Trim()
    if ($sentence.Length -eq 0) {
        return $Name
    }
    return $sentence.Substring(0, 1).ToUpperInvariant() + $sentence.Substring(1)
}

# The last segment of a case path is the function; what comes before it is the
# module the case sits in.
function Get-CaseName {
    param([string]$Path)

    $parts = @($Path -split "::")
    return $parts[$parts.Count - 1]
}

function Get-CaseModule {
    param([string]$Path)

    $parts = @($Path -split "::")
    if ($parts.Count -lt 2) {
        return ""
    }
    return ($parts[0..($parts.Count - 2)] -join "::")
}

function Get-TestCatalog {
    param([string]$RepoRoot, [string]$Language = "en-US")

    $documentLanguage = Get-ReportDocLanguage -Language $Language
    $catalog = @{}

    # The language's own TEST_COVERAGE.md is a table of behaviour against the
    # cases that hold it, so each row is read backwards into "this case protects
    # that". Its heading row names no case, so it adds no chip.
    $coveragePath = Join-Path $RepoRoot "docs/$documentLanguage/TEST_COVERAGE.md"
    if (Test-Path -LiteralPath $coveragePath -PathType Leaf) {
        foreach ($line in [System.IO.File]::ReadAllLines($coveragePath)) {
            if (-not $line.StartsWith("|")) {
                continue
            }
            $cells = @($line.Split("|") | Select-Object -Skip 1)
            if ($cells.Count -lt 3) {
                continue
            }
            # The cell is markdown, so a configuration key arrives wrapped in
            # backticks: the chip shows the key, not the markup around it.
            $behaviour = $cells[0].Trim().Replace('`', '')
            $cases = $cells[1]
            if ($behaviour.Length -eq 0 -or $behaviour -match "^-+$" -or $behaviour -eq "Setting" -or $behaviour -eq "Behaviour") {
                continue
            }
            # A case is named by its function, or by the whole path it sits
            # at when it has a module of its own, so the name may carry "::".
            foreach ($match in [regex]::Matches($cases, '`([A-Za-z0-9_:]+)`')) {
                $name = $match.Groups[1].Value
                if (-not $catalog.ContainsKey($name)) {
                    $catalog[$name] = @{
                        Doc       = ""
                        Protects  = (New-Object System.Collections.Generic.List[string])
                        Source    = $false
                        Described = $false
                    }
                }
                if (-not $catalog[$name].Protects.Contains($behaviour)) {
                    $catalog[$name].Protects.Add($behaviour)
                }
            }
        }
    }

    # A doc comment sits directly above the case it describes, so it is read by
    # walking back from the function: doc lines, attributes, and nothing else.
    $sources = Join-Path $RepoRoot "crates"
    if (Test-Path -LiteralPath $sources -PathType Container) {
        $files = @(Get-ChildItem -Path $sources -Recurse -Filter *.rs -File | Sort-Object FullName)
        foreach ($file in $files) {
            $lines = [System.IO.File]::ReadAllLines($file.FullName)
            for ($index = 0; $index -lt $lines.Count; $index++) {
                if (-not ($lines[$index] -match '^\s*(?:pub\s+)?fn\s+([a-z0-9_]+)\s*\(')) {
                    continue
                }
                $name = $Matches[1]
                $comment = New-Object System.Collections.Generic.List[string]
                $isCase = $false
                for ($earlier = $index - 1; $earlier -ge 0; $earlier--) {
                    $trimmed = $lines[$earlier].Trim()
                    if ($trimmed.StartsWith("///")) {
                        $comment.Insert(0, $trimmed.Substring(3).Trim())
                    }
                    elseif ($trimmed.StartsWith("#[")) {
                        # The attribute that is the case itself, not a cfg(test)
                        # helper that happens to mention tests.
                        if ($trimmed -match '^#\[[A-Za-z_0-9:]*test\b') {
                            $isCase = $true
                        }
                    }
                    elseif ($trimmed.Length -ne 0) {
                        break
                    }
                }
                if (-not $isCase) {
                    continue
                }
                $text = (@($comment | Where-Object { $_.Length -gt 0 }) -join " ").Trim()
                if (-not $catalog.ContainsKey($name)) {
                    $catalog[$name] = @{
                        Doc       = ""
                        Protects  = (New-Object System.Collections.Generic.List[string])
                        Source    = $false
                        Described = $false
                    }
                }
                if ($text.Length -gt 0) {
                    $catalog[$name].Doc = $text
                }
                $catalog[$name].Source = $true
            }
        }
    }

    # The report's own language may keep a table of its own words for the cases,
    # one row per case, beside the coverage document. A case the table names is
    # described in the reader's language; a case it does not name keeps the doc
    # comment above it, or its own name. The table is read last on purpose: when
    # it and the sources both have something to say, the table is what a reader
    # of that language is shown.
    $casesPath = Join-Path $RepoRoot "docs/$documentLanguage/TEST_CASES.md"
    if (Test-Path -LiteralPath $casesPath -PathType Leaf) {
        foreach ($line in [System.IO.File]::ReadAllLines($casesPath)) {
            if (-not $line.StartsWith("|")) {
                continue
            }
            $cells = @($line.Split("|") | Select-Object -Skip 1)
            if ($cells.Count -lt 3) {
                continue
            }
            $named = [regex]::Match($cells[0], '`([A-Za-z0-9_:]+)`')
            $described = $cells[1].Trim()
            if (-not $named.Success -or $described.Length -eq 0) {
                continue
            }
            $name = $named.Groups[1].Value
            if (-not $catalog.ContainsKey($name)) {
                $catalog[$name] = @{
                    Doc       = ""
                    Protects  = (New-Object System.Collections.Generic.List[string])
                    Source    = $false
                    Described = $false
                }
            }
            $catalog[$name].Doc = $described
            $catalog[$name].Described = $true
        }
    }

    return $catalog
}

# One cell of a report table: its text, and the class, tooltip, link or second
# line that cell carries.
function New-ReportCell {
    param([string]$Text, [string]$Class = "", [string]$Title = "", [string]$Link = "", [string]$Sub = "")

    $cell = @{ Text = $Text }
    if ($Class) { $cell["Class"] = $Class }
    if ($Title) { $cell["Title"] = $Title }
    if ($Link) { $cell["Link"] = $Link }
    if ($Sub) { $cell["Sub"] = $Sub }
    return $cell
}

# The coverage document's layer table: what the cases of a layer are for, and
# what passing them does not prove. It is read the way the behaviour tables are,
# and it is the only four-column table in the document, so a row is a layer row
# when it has four cells and its second one carries the count the layer
# declares; the heading above them declares none.
function Get-TestLayerCatalog {
    param([string]$RepoRoot, [string]$Language = "en-US")

    $layers = New-Object System.Collections.Generic.List[object]
    $documentLanguage = Get-ReportDocLanguage -Language $Language
    $coveragePath = Join-Path $RepoRoot "docs/$documentLanguage/TEST_COVERAGE.md"
    if (-not (Test-Path -LiteralPath $coveragePath -PathType Leaf)) {
        return @()
    }
    $started = $false
    foreach ($line in [System.IO.File]::ReadAllLines($coveragePath)) {
        if (-not $line.StartsWith("|")) {
            if ($started) { break }
            continue
        }
        $cells = @($line.Split("|") | Select-Object -Skip 1)
        if ($cells.Count -ne 5) {
            continue
        }
        $name = $cells[0].Trim()
        $count = $cells[1].Trim()
        if (-not $started) {
            if ($count -notmatch "\d") { continue }
            $started = $true
        }
        if ($name.Length -eq 0 -or $name -match "^-+$") {
            continue
        }
        $layers.Add(@{
            Name        = $name
            Count       = $count
            Proves      = $cells[2].Trim()
            CannotProve = $cells[3].Trim()
        })
    }
    return $layers.ToArray()
}

# A coverage document is markdown, and a report is not: a link keeps the words
# it shows, a name inside backticks keeps the name, and the emphasis markers go.
function ConvertFrom-CoverageText {
    param([string]$Text)

    $plain = [regex]::Replace($Text, '\[([^\]]+)\]\([^)]+\)', '$1')
    $plain = $plain.Replace('`', '')
    $plain = $plain.Replace('**', '')
    return $plain.Trim()
}

# The document read the other way round: every behaviour it promises and the
# cases it names for that behaviour, plus the part of it that admits no case
# covers a thing. A report that lists a behaviour's cases beside their outcome
# says what was tested, rather than only what passed.
#
# The document is written for a reader, so a behaviour row is one whose cases
# are named in backticks and whose first cell is neither a heading nor a rule;
# its behaviour cell may name several settings at once and its case cell may
# describe them in prose, so only the backticked names are taken. A behaviour
# table row splits into three cells and a layer table row into five, so the
# layer table -- which the report shows on its own -- is not read twice.
function Get-TestCoverage {
    param([string]$RepoRoot, [string]$Language = "en-US")

    $rows = New-Object System.Collections.Generic.List[object]
    $groups = New-Object System.Collections.Generic.List[object]
    $documentLanguage = Get-ReportDocLanguage -Language $Language
    $coveragePath = Join-Path $RepoRoot "docs/$documentLanguage/TEST_COVERAGE.md"
    if (-not (Test-Path -LiteralPath $coveragePath -PathType Leaf)) {
        return @{ Rows = @(); Uncovered = @() }
    }

    $section = ""
    # A bullet of the uncovered part, and the group of bullets it belongs to:
    # the document wraps a long bullet onto an indented line of its own.
    $group = $null
    $item = -1
    foreach ($line in [System.IO.File]::ReadAllLines($coveragePath)) {
        if ($line.StartsWith("## ")) {
            $section = ConvertFrom-CoverageText -Text $line.Substring(3)
            $group = $null
            $item = -1
            continue
        }
        if ($line -match "^- ") {
            if ($null -eq $group -or $group.Section -ne $section) {
                $group = @{ Section = $section; Lines = (New-Object System.Collections.Generic.List[string]) }
                $groups.Add($group)
            }
            $group.Lines.Add((ConvertFrom-CoverageText -Text $line.Substring(2)))
            $item = $group.Lines.Count - 1
            continue
        }
        if ($null -ne $group -and $item -ge 0 -and $line.StartsWith("  ") -and $line.Trim().Length -gt 0) {
            $group.Lines[$item] = "$($group.Lines[$item]) $(ConvertFrom-CoverageText -Text $line)"
            continue
        }
        if (-not $line.StartsWith("|")) {
            $group = $null
            $item = -1
            continue
        }
        $cells = @($line.Split("|") | Select-Object -Skip 1)
        if ($cells.Count -ne 3) {
            continue
        }
        if (-not $cells[1].Contains('`')) {
            continue
        }
        $behaviour = ConvertFrom-CoverageText -Text $cells[0]
        if ($behaviour.Length -eq 0) {
            continue
        }
        $names = New-Object System.Collections.Generic.List[string]
        foreach ($match in [regex]::Matches($cells[1], '`([A-Za-z0-9_:]+)`')) {
            $names.Add($match.Groups[1].Value)
        }
        $rows.Add(@{ Section = $section; Behaviour = $behaviour; Cases = $names.ToArray() })
    }
    return @{ Rows = $rows.ToArray(); Uncovered = $groups.ToArray() }
}

# Which layer of the suite a target belongs to. A result line names the binary
# that ran the cases, the binary is named after the crate, and the crate is what
# says which layer a target is: the core library, the setup end-to-end cases,
# the project inspection, the visual builder, or an extraction runtime. A binary
# the coverage document gives no layer -- the workspace also builds the command
# line tool and runs doc-tests -- is a target of its own rather than a guess.
function Get-TargetLayerKey {
    param([string]$Target)

    $crate = $Target
    if ($Target -match "\(([^)]+)\)") {
        $crate = $Matches[1]
    }
    $crate = Split-Path -Leaf $crate
    $crate = $crate -replace "-[0-9a-f]{6,}\.exe$", ""
    if ($crate -like "*nano_installer_core*") { return "core" }
    if ($crate -like "*e2e_setup*") { return "e2e" }
    if ($crate -like "*project_inspection*") { return "inspection" }
    if ($crate -like "*nano_installer_gui*") { return "gui" }
    if ($crate -like "*lzma_stub_native*" -or $crate -like "*zlib_stub_native*") { return "runtime" }
    return "unknown"
}

# The layers a report explains: the rows the coverage document gives the layers
# this run has targets in, and a row of the report's own words for a layer the
# document does not name, so no target is left without one.
function Get-ReportLayerTable {
    param([hashtable]$Text, [object[]]$Document, [string[]]$Keys = @())

    $layers = New-Object System.Collections.Generic.List[object]
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($key in $Keys) {
        $names.Add((Get-ReportPhrase -Text $Text -Key "layer.$key"))
    }
    foreach ($row in $Document) {
        if ($names.Contains($row.Name)) {
            $layers.Add(@{
                Anchor      = "layer-$($layers.Count)"
                Name        = $row.Name
                Count       = $row.Count
                Proves      = $row.Proves
                CannotProve = $row.CannotProve
            })
        }
    }
    foreach ($name in $names) {
        $known = $false
        foreach ($layer in $layers) {
            if ($layer.Name -eq $name) { $known = $true }
        }
        if (-not $known) {
            $layers.Add(@{
                Anchor      = "layer-$($layers.Count)"
                Name        = $name
                Count       = ""
                Proves      = (Get-ReportPhrase -Text $Text -Key "layers.nameless")
                CannotProve = ""
            })
        }
    }
    return $layers.ToArray()
}

# Where each layer's row sits, so a target can point at what its layer proves.
function Get-ReportLayerAnchor {
    param([object[]]$Layers)

    $anchors = @{}
    foreach ($layer in $Layers) {
        $anchors[$layer.Name] = "#$($layer.Anchor)"
    }
    return $anchors
}

# One row for a cases table: what the case is called, where it sits, and what it
# holds. The doc comment above the case says what it checks; a case that has none
# is described by its own name, so no row is ever blank.
function Get-CaseRow {
    param([hashtable]$Catalog, [string]$Path, [string]$Result, [string]$Note)

    $name = Get-CaseName -Path $Path
    $what = ConvertTo-CaseSentence -Name $name
    $protects = @()
    $described = $false
    if ($Catalog.ContainsKey($name)) {
        if ($Catalog[$name].Doc) { $what = $Catalog[$name].Doc }
        if ($Catalog[$name].Described) { $described = $true }
        $protects = @($Catalog[$name].Protects)
    }
    return @{
        Name      = $name
        Module    = Get-CaseModule -Path $Path
        Result    = $Result
        Note      = $Note
        What      = $what
        Protects  = $protects
        Described = $described
    }
}

# Every case a run reported, by the names a coverage row can call it by: the
# document names a case by its function or by the whole path it sits at, so both
# are recorded, and a case that failed is never overwritten by one that passed.
# Which layer ran the case is kept with it, because a case name says nothing
# about whether the library was driven in process or a built setup was run.
function Add-CaseResult {
    param([hashtable]$Results, [string]$Path, [string]$Result, [string]$Layer = "")

    foreach ($key in @((Get-CaseName -Path $Path), $Path)) {
        if (-not $key) { continue }
        if ($Results.ContainsKey($key) -and $Results[$key].Result -eq "FAILED") { continue }
        $Results[$key] = @{ Result = $Result; Layer = $Layer }
    }
}

# A documented behaviour, with what became of the cases that hold it in this
# run. One failed case fails the behaviour whatever else ran; a behaviour whose
# cases ran and passed is what the run proved; a behaviour whose only cases were
# skipped was not proven by them; and one whose cases never ran at all keeps an
# empty result rather than being left off the page.
function Get-BehaviourRows {
    param([object[]]$Coverage, [hashtable]$Results)

    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($row in $Coverage) {
        $cases = New-Object System.Collections.Generic.List[object]
        $ok = 0
        $bad = 0
        $skip = 0
        foreach ($name in $row.Cases) {
            $result = ""
            $layer = ""
            if ($Results.ContainsKey($name)) {
                $result = $Results[$name].Result
                $layer = $Results[$name].Layer
            }
            if ($result -eq "ok") { $ok++ }
            elseif ($result -eq "FAILED") { $bad++ }
            elseif ($result -eq "ignored") { $skip++ }
            $cases.Add(@{ Name = $name; Result = $result; Layer = $layer })
        }
        $verdict = ""
        if ($bad -gt 0) { $verdict = "FAILED" }
        elseif ($ok -gt 0) { $verdict = "ok" }
        elseif ($skip -gt 0) { $verdict = "ignored" }
        $rows.Add(@{
            Section   = $row.Section
            Behaviour = $row.Behaviour
            Verdict   = $verdict
            Cases     = $cases.ToArray()
        })
    }
    return $rows.ToArray()
}

# The behaviours a run has something to say about: the ones it ran a case for.
# A suite that holds one layer of the document leaves the rest of it to the
# other suites rather than listing it as its own failure to run.
function Select-BehaviourRow {
    param([object[]]$Rows)

    $kept = New-Object System.Collections.Generic.List[object]
    foreach ($row in $Rows) {
        if ($row.Verdict) { $kept.Add($row) }
    }
    return $kept.ToArray()
}

# The pages of a real setup, as capture_setup_snapshots.ps1 photographed them.
# The manifest names every page the project declares and the layout each page is
# drawn from, and every snapshot says which page it is, so a page is shown under
# the name the project gives it. Each check the capture made is recorded by name
# with its measurements rather than as a finished sentence, and the sentence is
# made here in the report's language. The image travels inside the page, so the
# report is one file a reader can open wherever it is put.
function Get-SnapshotGallery {
    param([string]$Directory, [hashtable]$Text = $null, [string]$Language = "en-US")

    if ($null -eq $Text) {
        $Text = Get-ReportText -Language "en-US"
    }
    $gallery = New-Object System.Collections.Generic.List[object]
    if (-not $Directory -or -not (Test-Path -LiteralPath $Directory -PathType Container)) {
        return @()
    }
    $manifestPath = Join-Path $Directory "manifest.json"
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        return @()
    }

    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json
    $project = "the project"
    if ($manifest.PSObject.Properties.Name -contains "project") {
        $project = [string]$manifest.project
    }
    $pages = @{}
    foreach ($page in $manifest.pages) {
        $pages[[string]$page.id] = $page
    }
    foreach ($snapshot in $manifest.snapshots) {
        $file = Join-Path $Directory ([string]$snapshot.file)
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            continue
        }
        # The page a snapshot belongs to carries the name a reader knows it by;
        # a page the manifest does not name keeps the wizard's first.
        $id = [string]$snapshot.page
        $role = "install"
        $index = 1
        $title = ""
        $layout = ""
        if ($pages.ContainsKey($id)) {
            $role = [string]$pages[$id].role
            $index = [int]$pages[$id].index
            $title = [string]$pages[$id].title
            $layout = [string]$pages[$id].layout
        }
        $checks = New-Object System.Collections.Generic.List[string]
        foreach ($check in $snapshot.asserted) {
            $key = "check.$([string]$check.k)"
            if (-not $Text.ContainsKey($key)) {
                continue
            }
            # A check that measured nothing -- the corners, say -- carries no
            # value, and the sentence for it has no slot to fill.
            $values = @()
            if ($check.PSObject.Properties.Name -contains "v") {
                $values = @($check.v)
            }
            $checks.Add((Get-ReportPhrase -Text $Text -Key $key -Values $values))
        }
        # What the page must show is the report's own words about that layout,
        # rather than the prompt the capture wrote for a vision model.
        $expectation = ""
        if ($layout) {
            $expectationKey = "expectation.$([System.IO.Path]::GetFileNameWithoutExtension($layout))"
            if ($Text.ContainsKey($expectationKey)) {
                $expectation = $Text[$expectationKey]
            }
        }
        $roleName = Get-ReportPhrase -Text $Text -Key "snapshots.role.$role"
        $gallery.Add(@{
            File        = [string]$snapshot.file
            PageId      = $id
            Heading     = Get-ReportPhrase -Text $Text -Key "snapshots.page.heading" -Values @($roleName, $index, $title)
            Layout      = $layout
            Alt         = Get-ReportPhrase -Text $Text -Key "snapshots.alt" -Values @(
                $project,
                (Get-ReportPhrase -Text $Text -Key "snapshots.page" -Values @($roleName, $index, $title)),
                $layout, $snapshot.locale, $snapshot.scaling, $snapshot.width, $snapshot.height)
            Source      = "data:image/png;base64,$([Convert]::ToBase64String([System.IO.File]::ReadAllBytes($file)))"
            Caption     = Get-ReportPhrase -Text $Text -Key "snapshots.caption" -Values @($snapshot.locale, $layout, $snapshot.width, $snapshot.height, $snapshot.scaling)
            Checks      = $checks.ToArray()
            Expectation = $expectation
        })
    }
    return $gallery.ToArray()
}

# The cases of a run that the language's own table of case descriptions does not
# name. A report whose column of case descriptions fell back to the sources'
# English for some of its rows says so, since a reader of that language cannot
# tell an English sentence from a translated one at a glance.
function Get-UndescribedCaseNote {
    param([hashtable]$Text, [string]$RepoRoot, [string]$Language, [object[]]$Cases)

    $documentLanguage = Get-ReportDocLanguage -Language $Language
    $path = "docs/$documentLanguage/TEST_CASES.md"
    if (-not $Cases -or $Cases.Count -eq 0) {
        return ""
    }
    if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot $path) -PathType Leaf)) {
        return ""
    }
    $undescribed = 0
    foreach ($case in $Cases) {
        if (-not $case.Described) { $undescribed++ }
    }
    if ($undescribed -eq 0) {
        return ""
    }
    return (Get-ReportPhrase -Text $Text -Key "cases.notdescribed" -Values @($undescribed, $path))
}
