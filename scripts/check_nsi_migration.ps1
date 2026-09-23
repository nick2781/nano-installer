<#
    Reads an NSIS script and says what each of its lines becomes here.

    Migrating an installer means going through a script someone else wrote and
    deciding, line by line, whether this project already does that. Most of the
    answer is mechanical: `WriteRegStr` is `reg_write_string`, `MUI_PAGE_FINISH`
    is the page the configuration gives `role: "finish"`, `SetCompressor` is
    nothing at all because the builder packs the payload itself. The rest is the
    part worth reading twice, and this script is here to put it in front of you
    rather than let it hide in a thousand-line file.

    The table this script classifies with is the table in
    docs/en/MIGRATION_FROM_NSIS.md and docs/zh-CN/MIGRATION_FROM_NSIS.md, read
    straight out of the two documents. That is deliberate: a guide and a tool
    that each carry their own copy of a hundred rows drift apart, and the one
    that drifts is the one nobody runs. Both guides are read and have to agree --
    same commands, same verdicts, whatever language the prose is in -- so a row
    added to one language and not the other fails this check. A statement the
    table has no row for is reported as `unknown` rather than guessed at.

    A row is a table row whose second cell is one of the five verdicts, so the
    prose tables above and below it are ignored. `*` at the end of a command
    matches anything after it, which is how `MUI_PAGE_*` catches the MUI pages
    that have no row of their own.

    Usage:
        .\scripts\check_nsi_migration.ps1 -Script .\examples\nsis-migration\legacy.nsi
        .\scripts\check_nsi_migration.ps1 -Script .\legacy.nsi -Language zh-CN
        .\scripts\check_nsi_migration.ps1 -Script .\legacy.nsi -FailOnUnknown

    -Guide points at another documentation tree with the same shape, which is how
    the agreement between the two languages is exercised without editing either
    of them.

    The report speaks English, like the rest of scripts/, and every line of it is
    ASCII except the table's own words, so Windows PowerShell decodes it the same
    way on any machine. -Language picks which guide's words those are.

    The exit code is 0 whenever a report was produced, whatever the report says.
    A script that cannot be read, or two guides that disagree, is an error, and
    -FailOnUnknown is for a pipeline that wants the guide to cover the file it
    checks: with the switch, a statement the table does not know fails the run.
#>
param(
    [string]$Script,
    [string]$Language = "en",
    [string]$Guide,
    [switch]$FailOnUnknown
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $Guide) {
    $Guide = Join-Path $repoRoot "docs"
}
# The guide in both languages, under the documentation tree the build already
# reads. -Guide points at another tree of the same shape, which is how the
# agreement check between the two languages can be exercised.
$paths = @(
    (Join-Path $Guide "en/MIGRATION_FROM_NSIS.md"),
    (Join-Path $Guide "zh-CN/MIGRATION_FROM_NSIS.md")
)

# What each verdict means, printed with the report so a reader does not have to
# hold the list in their head. These five words are also what marks a table row
# as a row of the mapping: a row whose second cell is one of them is read, and
# every other table in the two guides is prose.
$Verdicts = [ordered]@{
    direct  = "a configuration setting or a page element does this"
    script  = "a primitive in scripts/install.rhai or scripts/uninstall.rhai does this"
    manual  = "no equivalent: the guide says what to do instead"
    none    = "build-time or cosmetic: nothing to migrate"
    unknown = "the guides' table has no row for this yet"
}

# A script, read the way the rest of this repository reads one: an explicit BOM
# first, then UTF-8 if the bytes are valid UTF-8, then the machine's own code
# page, which is what an NSIS script written years ago is most likely to be.
function Read-NsiText {
    param([string]$Path)

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        return [System.Text.Encoding]::UTF8.GetString($bytes, 3, $bytes.Length - 3)
    }
    if ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) {
        return [System.Text.Encoding]::Unicode.GetString($bytes, 2, $bytes.Length - 2)
    }
    if ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFE -and $bytes[1] -eq 0xFF) {
        return [System.Text.Encoding]::BigEndianUnicode.GetString($bytes, 2, $bytes.Length - 2)
    }
    try {
        $strict = New-Object System.Text.UTF8Encoding($false, $true)
        return $strict.GetString($bytes)
    }
    catch {
        return [System.Text.Encoding]::Default.GetString($bytes)
    }
}

# The mapping tables of one guide, as command -> verdict + what it becomes.
function Read-GuideTable {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "migration guide not found: $Path"
    }
    $rows = @{}
    $duplicates = New-Object System.Collections.Generic.List[string]
    $lines = ([System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)) -split "\r?\n"
    foreach ($line in $lines) {
        $trimmed = $line.Trim()
        if (-not $trimmed.StartsWith("|")) {
            continue
        }
        $parts = $trimmed.Split('|')
        if ($parts.Count -lt 5) {
            # A two- or three-cell row: the verdict column is not there, so this
            # is one of the prose tables.
            continue
        }
        $cells = @()
        for ($index = 1; $index -lt $parts.Count - 1; $index++) {
            $cells += $parts[$index].Trim()
        }
        if ($cells.Count -lt 3) {
            continue
        }
        $verdict = $cells[1]
        if (-not $Verdicts.Contains($verdict)) {
            continue
        }
        $becomes = $cells[2].Trim()
        foreach ($command in ($cells[0] -split ',')) {
            $name = $command.Trim().Trim('`').Trim()
            if ($name -eq '') {
                continue
            }
            $key = $name.ToLower()
            if ($rows.ContainsKey($key)) {
                $duplicates.Add($name)
                continue
            }
            $rows[$key] = @{
                Name    = $name
                Verdict = $verdict
                Becomes = $becomes
            }
        }
    }
    if ($duplicates.Count -gt 0) {
        throw ("$Path has more than one row for: " + (($duplicates | Sort-Object -Unique) -join ', '))
    }
    return $rows
}

# Both guides, and the two of them have to say the same thing. A row that only
# one language carries would otherwise be a command this tool knows about in one
# language and not in the other.
$tables = @{}
foreach ($path in $paths) {
    $tables[$path] = Read-GuideTable -Path $path
}
$reference = $paths[0]
foreach ($path in $paths) {
    $other = $tables[$path]
    $base = $tables[$reference]
    $missing = @($base.Keys | Where-Object { -not $other.ContainsKey($_) })
    $extra = @($other.Keys | Where-Object { -not $base.ContainsKey($_) })
    if ($missing.Count -gt 0) {
        throw "$path has no row for: $(($missing | Sort-Object) -join ', ')"
    }
    if ($extra.Count -gt 0) {
        throw "$reference has no row for: $(($extra | Sort-Object) -join ', ')"
    }
    foreach ($key in $base.Keys) {
        if ($base[$key].Verdict -ne $other[$key].Verdict) {
            throw "$($base[$key].Name) is $($base[$key].Verdict) in $reference and $($other[$key].Verdict) in $path"
        }
    }
}

# Which guide's words the report shows. The verdicts and the commands are the
# same either way; only the sentence beside them changes.
$words = $tables[$reference]
foreach ($path in $paths) {
    if ((Split-Path -Leaf (Split-Path -Parent $path)) -eq $Language) {
        $words = $tables[$path]
    }
}

# A line without its comment. NSIS ends a comment at `;` or `#`, both of which
# are ordinary characters inside a quoted argument.
function Get-Statement {
    param([string]$Line)

    $quoted = $false
    for ($index = 0; $index -lt $Line.Length; $index++) {
        $character = $Line[$index]
        if ($character -eq '"') {
            $quoted = -not $quoted
        }
        elseif (-not $quoted -and ($character -eq ';' -or $character -eq '#')) {
            return $Line.Substring(0, $index).Trim()
        }
    }
    return $Line.Trim()
}

# The row a command matches, the longest pattern first so that a row written for
# one command wins over the wildcard that would also catch it.
function Get-Row {
    param([string]$Name)

    $key = $Name.ToLower()
    if ($words.ContainsKey($key)) {
        return $words[$key]
    }
    $match = $null
    foreach ($pattern in $words.Keys) {
        if ($pattern.Contains('*') -and $key -like $pattern) {
            if ($null -eq $match -or $pattern.Length -gt $match.Length) {
                $match = $pattern
            }
        }
    }
    if ($null -ne $match) {
        return $words[$match]
    }
    return $null
}

# Headers NSIS ships itself: including them needs nothing from the project.
$StockHeaders = @(
    "mui2.nsh", "mui.nsh", "logiclib.nsh", "filefunc.nsh", "winver.nsh", "sections.nsh",
    "x64.nsh", "strfunc.nsh", "wordfunc.nsh", "textfunc.nsh", "nsdialogs.nsh",
    "gti.nsh", "installoptions.nsh", "util.nsh", "nsisconf.nsh"
)

# What one statement becomes. The first word decides, except for the two forms
# whose name is not their first word: !insertmacro is looked up by its macro, and
# !include is followed rather than classified.
function Get-Verdict {
    param([string]$Statement)

    if ($Statement -like '!insertmacro*') {
        $rest = $Statement.Substring(12).Trim()
        $macro = ($rest -split '\s+')[0]
        if ($macro -ne '') {
            $row = Get-Row -Name $macro
            if ($null -ne $row) {
                return @($row.Verdict, $row.Becomes)
            }
        }
    }

    $command = ($Statement -split '\s+')[0]
    $row = Get-Row -Name $command
    if ($null -ne $row) {
        if ($row.Verdict -eq 'unknown') {
            return @('unknown', $Verdicts['unknown'])
        }
        return @($row.Verdict, $row.Becomes)
    }
    return @('unknown', $Verdicts['unknown'])
}

if (-not $Script) {
    throw "usage: check_nsi_migration.ps1 -Script <file.nsi> [-Language en] [-FailOnUnknown]"
}
if (-not (Test-Path -LiteralPath $Script -PathType Leaf)) {
    throw "NSIS script not found: $Script"
}
$entry = (Resolve-Path -LiteralPath $Script).Path
$root = Split-Path -Parent $entry

$counts = [ordered]@{}
foreach ($verdict in $Verdicts.Keys) {
    $counts[$verdict] = 0
}
$attention = New-Object System.Collections.Generic.List[string]
$visited = New-Object System.Collections.Generic.HashSet[string]
$queue = New-Object System.Collections.Generic.Queue[object]
$queue.Enqueue(@{ Path = $entry; Display = (Split-Path -Leaf $entry); Depth = 0 })
$visited.Add($entry.ToLower()) | Out-Null
$reported = 0

Write-Output "NSIS migration report for $entry"
Write-Output ""
Write-Output "the table was read from $($paths -join ' and ') ($($words.Count) command(s))"
Write-Output ""
foreach ($verdict in $Verdicts.Keys) {
    Write-Output ("  {0,-8} {1}" -f $verdict, $Verdicts[$verdict])
}
Write-Output ""

while ($queue.Count -gt 0) {
    $file = $queue.Dequeue()
    $lines = (Read-NsiText -Path $file.Path) -split "\r?\n"
    for ($index = 0; $index -lt $lines.Count; $index++) {
        $statement = Get-Statement -Line $lines[$index]
        if ($statement -eq '') {
            continue
        }
        if ($statement -like '!include*') {
            # An !include of a file next to the script is followed, so its
            # statements are reported too. A header NSIS ships itself, or a file
            # that is not there, is reported where it stands.
            $name = $statement.Substring(8).Trim().Trim('"').Trim()
            $leaf = Split-Path -Leaf ($name -replace '\$NSISDIR', '')
            $target = Join-Path (Split-Path -Parent $file.Path) $name
            if ($leaf -ne '' -and $StockHeaders -contains $leaf.ToLower()) {
                $verdict = @('none', 'an NSIS header, nothing to migrate')
            }
            elseif (Test-Path -LiteralPath $target -PathType Leaf) {
                if ($file.Depth -ge 8) {
                    $verdict = @('manual', 'includes are nested more than 8 deep; the rest of that file was not read')
                }
                elseif ($visited.Add((Resolve-Path -LiteralPath $target).Path.ToLower())) {
                    $resolved = (Resolve-Path -LiteralPath $target).Path
                    $queue.Enqueue(@{
                        Path    = $resolved
                        Display = $resolved.Substring($root.Length).TrimStart('\')
                        Depth   = $file.Depth + 1
                    })
                    $verdict = @('none', 'an included file, reported on its own lines below')
                }
                else {
                    $verdict = @('none', 'already read')
                }
            }
            else {
                $verdict = @('manual', 'the included file is not next to this script')
            }
        }
        else {
            $verdict = Get-Verdict -Statement $statement
        }
        $counts[$verdict[0]] = $counts[$verdict[0]] + 1
        $reported++
        $shown = $statement
        if ($shown.Length -gt 52) {
            $shown = $shown.Substring(0, 49) + "..."
        }
        Write-Output ("  {0,5}  {1,-52}  {2,-7}  {3}" -f ($index + 1), $shown, $verdict[0], $verdict[1])
        if ($verdict[0] -eq 'manual' -or $verdict[0] -eq 'unknown') {
            $attention.Add(("{0}:{1}  {2}" -f $file.Display, ($index + 1), $statement))
        }
    }
}

Write-Output ""
Write-Output "$reported statement(s) read from $($visited.Count) file(s)"
foreach ($verdict in $Verdicts.Keys) {
    Write-Output ("  {0,-8} {1,4}" -f $verdict, $counts[$verdict])
}

if ($attention.Count -gt 0) {
    Write-Output ""
    Write-Output "$($attention.Count) line(s) the guide has to answer for:"
    foreach ($line in $attention) {
        Write-Output "  $line"
    }
}

$unknown = $counts['unknown']
if ($FailOnUnknown -and $unknown -gt 0) {
    throw "$unknown statement(s) have no row in the migration guide's table"
}

Write-Output ""
Write-Output "NSIS migration check finished: $reported statement(s), $unknown with no row yet"
