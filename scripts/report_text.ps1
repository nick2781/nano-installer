<#
    The words a report says, per language, and the few shapes they are put in.

    A report is read by whoever ran the suite, and this project writes its
    documents in two languages, so the page and the text file are written in the
    language they are run with. The words live in report_text.json, which is
    UTF-8: a script file may only hold non-ASCII text when it carries a UTF-8
    BOM, and a data file says what it is without that trap.

    English is the base. A translation is checked against it, so a language that
    is missing a word, or that dropped a placeholder from one, fails where it is
    used instead of printing an empty row.

    The names of cases, the doc comments above them and the output of the tools
    are not here: they are what the sources and the tools wrote, and a report
    shows them as they are.
#>

function Get-ReportText {
    param([string]$Language = "en-US")

    $path = Join-Path $PSScriptRoot "report_text.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Report text not found: $path"
    }
    $table = Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
    $base = $table.PSObject.Properties["en-US"]
    if ($null -eq $base) {
        throw "Report text holds no en-US strings: $path"
    }
    $chosen = $table.PSObject.Properties[$Language]
    if ($null -eq $chosen) {
        $known = @($table.PSObject.Properties.Name) -join ", "
        throw "Report text has no $Language, only: $known"
    }

    $text = @{}
    foreach ($property in $base.Value.PSObject.Properties) {
        $text[$property.Name] = [string]$property.Value
    }

    $unknown = New-Object System.Collections.Generic.List[string]
    $wrong = New-Object System.Collections.Generic.List[string]
    foreach ($property in $chosen.Value.PSObject.Properties) {
        if (-not $text.ContainsKey($property.Name)) {
            $unknown.Add($property.Name)
        }
        $baseSlots = @([regex]::Matches($text[$property.Name], "\{\d+\}") | ForEach-Object { $_.Value }) -join ","
        $translatedSlots = @([regex]::Matches([string]$property.Value, "\{\d+\}") | ForEach-Object { $_.Value }) -join ","
        if ($baseSlots -ne $translatedSlots) {
            $wrong.Add("$($property.Name) ($baseSlots -> $translatedSlots)")
        }
        $text[$property.Name] = [string]$property.Value
    }
    if ($unknown.Count -gt 0) {
        throw "$Language holds no English word for: $($unknown -join ', ')"
    }
    if ($wrong.Count -gt 0) {
        throw "$Language says different placeholders than en-US for: $($wrong -join '; ')"
    }

    return $text
}

# One string from the table, with its placeholders filled in.
function Get-ReportPhrase {
    param([hashtable]$Text, [string]$Key, [object[]]$Values = @())

    if (-not $Text.ContainsKey($Key)) {
        throw "Report text has no key: $Key"
    }
    if ($Values.Count -eq 0) {
        return $Text[$Key]
    }
    return [string]::Format($Text[$Key], $Values)
}

# A run's counts as one phrase. Every case that passed is always named -- a
# target that failed all of its cases still says how many passed, which is none
# -- while the other two are named when they happened, and -All names all three.
function Get-ReportCounts {
    param([hashtable]$Text, [int]$Passed, [int]$Failed, [int]$Ignored, [switch]$All)

    $separator = Get-ReportPhrase -Text $Text -Key "counts.separator"
    $parts = New-Object System.Collections.Generic.List[string]
    $parts.Add((Get-ReportPhrase -Text $Text -Key "counts.passed" -Values @($Passed)))
    if ($Failed -gt 0 -or $All) {
        $parts.Add((Get-ReportPhrase -Text $Text -Key "counts.failed" -Values @($Failed)))
    }
    if ($Ignored -gt 0 -or $All) {
        $parts.Add((Get-ReportPhrase -Text $Text -Key "counts.ignored" -Values @($Ignored)))
    }
    return ($parts -join $separator)
}

# The documents live under the name their own directory uses, which is not
# always the whole tag a page declares: the English pages are in docs/en.
function Get-ReportDocLanguage {
    param([string]$Language)

    $directories = @{ "en-US" = "en"; "zh-CN" = "zh-CN" }
    if ($directories.ContainsKey($Language)) {
        return $directories[$Language]
    }
    return $Language
}

# One word for a case's outcome, whatever shape the reader meets it in: the tool
# prints "ok", "FAILED" or "ignored", and the table says the same outcome in the
# report's language.
function Get-ReportResultLabel {
    param([hashtable]$Text, [string]$Result)

    if ($Result -eq "FAILED") { return Get-ReportPhrase -Text $Text -Key "stats.failed" }
    if ($Result -eq "ignored") { return Get-ReportPhrase -Text $Text -Key "stats.ignored" }
    return Get-ReportPhrase -Text $Text -Key "stats.passed"
}

# The plain text report lines its fields up in a column, and its labels are
# English or Chinese. A Chinese character takes two columns of a monospaced
# console and a Latin one takes a single column, so a label is measured by what
# it costs to read rather than by how many characters it is spelled with.
function Get-ReportLabelWidth {
    param([string]$Label)

    $columns = 0
    foreach ($character in $Label.ToCharArray()) {
        if ([int]$character -gt 0x7F) { $columns += 2 }
        else { $columns++ }
    }
    return $columns
}

function Add-ReportField {
    param(
        [System.Collections.Generic.List[string]]$Lines,
        [string]$Label,
        [string]$Value,
        [int]$Width = 12
    )

    $padding = $Width - (Get-ReportLabelWidth -Label $Label)
    if ($padding -lt 1) { $padding = 1 }
    $gap = ' ' * $padding
    $Lines.Add("$Label$gap$Value")
}
