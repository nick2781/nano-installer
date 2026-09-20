<#
    Reads what a report shows about the cases and the pages behind a run.

    A report is worth reading when a row says what the case holds, not only that
    it passed, so both of these read the repository rather than the run:

    - Get-TestCatalog reads every case out of the Rust sources it can find under
      crates/: the doc comment above a #[test] says what the case checks, and
      the covered behaviours come from docs/<language>/TEST_COVERAGE.md, so the
      chips beside a case read in the report's language. The doc comments are
      the sources' own English and are shown as they are written. A case with no
      doc comment is described by its own name.
    - Get-SnapshotGallery reads the manifest capture_setup_snapshots.ps1 writes
      beside its page photographs, so a report can show the pages of a real
      setup and what the capture measured on each of them.
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
            foreach ($match in [regex]::Matches($cases, '`([a-z0-9_]+)`')) {
                $name = $match.Groups[1].Value
                if (-not $catalog.ContainsKey($name)) {
                    $catalog[$name] = @{ Doc = ""; Protects = (New-Object System.Collections.Generic.List[string]) }
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
                        if ($trimmed -match "\btest\b") {
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
                    $catalog[$name] = @{ Doc = ""; Protects = (New-Object System.Collections.Generic.List[string]) }
                }
                if ($text.Length -gt 0) {
                    $catalog[$name].Doc = $text
                }
            }
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
    if ($Catalog.ContainsKey($name)) {
        if ($Catalog[$name].Doc) { $what = $Catalog[$name].Doc }
        $protects = @($Catalog[$name].Protects)
    }
    return @{
        Name     = $name
        Module   = Get-CaseModule -Path $Path
        Result   = $Result
        Note     = $Note
        What     = $what
        Protects = $protects
    }
}

# The pages of a real setup, as capture_setup_snapshots.ps1 photographed them.
# The image travels inside the page, so the report is one file a reader can open
# wherever it is put.
function Get-SnapshotGallery {
    param([string]$Directory, [hashtable]$Text = $null)

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
    foreach ($snapshot in $manifest.snapshots) {
        $file = Join-Path $Directory ([string]$snapshot.file)
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            continue
        }
        $gallery.Add(@{
            File      = [string]$snapshot.file
            Alt       = Get-ReportPhrase -Text $Text -Key "snapshots.alt" -Values @($project, $snapshot.locale, $snapshot.scaling)
            Source    = "data:image/png;base64,$([Convert]::ToBase64String([System.IO.File]::ReadAllBytes($file)))"
            Caption   = Get-ReportPhrase -Text $Text -Key "snapshots.caption" -Values @($project, $snapshot.width, $snapshot.height, $snapshot.locale, $snapshot.scaling)
            Asserts   = @($snapshot.asserted)
            Expectation = [string]$snapshot.expectation
        })
    }
    return $gallery.ToArray()
}
