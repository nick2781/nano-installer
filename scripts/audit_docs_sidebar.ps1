<#
    Holds the sidebar and the two content trees to each other.

    A page reaches a reader through the sidebar in site/astro.config.ts. The
    English groups list primary-collection (`docs`) entry ids, and the Chinese
    groups point at the same pages through the `zh-CN` collection's mount path,
    in a `link` that starts with /zh-CN/. `src/lib/languages.ts` splits the one
    list by exactly those two things -- a `docs` href has no /zh-CN prefix, a
    `zh-CN` href has one -- so that is how this audit tells the two groups apart.

    Nimbus registers two authoring rules for this -- `nimbus/sidebar-entry` and
    `nimbus/orphan-page` -- and answers for neither here: neither is in the
    CLI's implemented rule set at the version this site installs
    (@cloudflare/nimbus-docs 0.17.0: `nimbus-docs lint --rule nimbus/sidebar-entry`
    refuses the code). The pages of this site are `.mdx`, which is what the walk
    every authoring rule is handed reads: `findMdxFiles()` in dist/cli/index.js
    is `walkFilesSync(dir, { extensions: [".mdx"] })`, and both `lint` and `check`
    call it. So the rules open these pages now, and the sidebar is still held to
    nothing upstream, because the two rules that would cover it are the two the
    CLI does not implement. This audit is that coverage. It reads
    site/astro.config.ts, the two content trees and nothing else, so it needs no
    build and no toolchain, and it runs on the pull request as well as on the
    push that publishes. Its other half, scripts/audit_docs_links.ps1, answers the
    `nimbus/internal-link` rule over the build's output, because an anchor can
    only be checked once the headings have been rendered.

    Both directions fail here:

      - every Markdown page anywhere under site/src/content/docs and
        site/src/content/zh-CN has to be listed in its own language's groups, or
        nothing in the sidebar leads to it;
      - every entry id and every `link` in the sidebar has to answer to a page
        that exists, or the row leads nowhere;
      - a page listed twice is refused, because two rows for one page is a
        sidebar that no longer says where the page is.

    Every sidebar entry is held to the site's own pages. A link that leaves the
    site fails here as well: the sidebar of a documentation site is its table of
    contents, and a destination nothing on this site serves cannot be checked.
    Two addresses answer for a page that is not named by its own path: a tree's
    root (`/` and `/zh-CN`) is that tree's `index.mdx`, and a directory
    (`/zh-CN/guides`) is the `index.mdx` inside it.

    The list is read out of the config rather than executed: `sidebar.items` is
    either an array literal or the name of a `const` the config assigns one to,
    and a group's own `items` is read the same way.

    Usage:
      .\scripts\audit_docs_sidebar.ps1
      .\scripts\audit_docs_sidebar.ps1 -RepositoryRoot C:\src\nano-installer
#>
param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$configPath = Join-Path $RepositoryRoot "site/astro.config.ts"
$englishDirectory = Join-Path $RepositoryRoot "site/src/content/docs"
$chineseDirectory = Join-Path $RepositoryRoot "site/src/content/zh-CN"

foreach ($path in @($configPath, $englishDirectory, $chineseDirectory)) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Not found: $path"
    }
}

# ---------------------------------------------------------------------------
# Reading the sidebar out of the TypeScript config.
#
# A real parser is out of reach here, and none is needed: `sidebar.items` is a
# literal list of literals. What is needed is to not be fooled by a bracket or a
# comma inside a string, which is what the small scanners below are for.
# ---------------------------------------------------------------------------

function Remove-ConfigComments([string]$Text) {
    $built = New-Object System.Text.StringBuilder
    $quote = [char]0
    $index = 0
    while ($index -lt $Text.Length) {
        $character = $Text[$index]
        if ($quote -ne [char]0) {
            [void]$built.Append($character)
            if ($character -eq '\') {
                if ($index + 1 -lt $Text.Length) {
                    [void]$built.Append($Text[$index + 1])
                    $index += 2
                    continue
                }
            }
            elseif ($character -eq $quote) {
                $quote = [char]0
            }
            $index++
            continue
        }
        if ($character -eq '"' -or $character -eq "'") {
            $quote = $character
            [void]$built.Append($character)
            $index++
            continue
        }
        if ($character -eq '/' -and $index + 1 -lt $Text.Length) {
            if ($Text[$index + 1] -eq '/') {
                while ($index -lt $Text.Length -and $Text[$index] -ne "`n") { $index++ }
                continue
            }
            if ($Text[$index + 1] -eq '*') {
                $index += 2
                while ($index + 1 -lt $Text.Length -and -not ($Text[$index] -eq '*' -and $Text[$index + 1] -eq '/')) { $index++ }
                $index += 2
                continue
            }
        }
        [void]$built.Append($character)
        $index++
    }
    return $built.ToString()
}

function Get-BracketSlice([string]$Text, [int]$Start) {
    # Returns the balanced `[`...`]` or `{`...`}` run that starts at $Start.
    $opening = $Text[$Start]
    $closing = if ($opening -eq '[') { ']' } else { '}' }
    $depth = 0
    $quote = [char]0
    $index = $Start
    while ($index -lt $Text.Length) {
        $character = $Text[$index]
        if ($quote -ne [char]0) {
            if ($character -eq '\') { $index += 2; continue }
            if ($character -eq $quote) { $quote = [char]0 }
            $index++
            continue
        }
        if ($character -eq '"' -or $character -eq "'") { $quote = $character; $index++; continue }
        if ($character -eq $opening) { $depth++ }
        elseif ($character -eq $closing) {
            $depth--
            if ($depth -eq 0) {
                return @{ Text = $Text.Substring($Start, $index - $Start + 1); Next = $index + 1 }
            }
        }
        $index++
    }
    throw "site/astro.config.ts has an unbalanced '$opening'"
}

function Split-TopLevel([string]$Text) {
    # Splits on the commas that belong to this list, not to a nested one.
    $parts = New-Object System.Collections.Generic.List[string]
    $depth = 0
    $quote = [char]0
    $start = 0
    $index = 0
    while ($index -lt $Text.Length) {
        $character = $Text[$index]
        if ($quote -ne [char]0) {
            if ($character -eq '\') { $index += 2; continue }
            if ($character -eq $quote) { $quote = [char]0 }
            $index++
            continue
        }
        if ($character -eq '"' -or $character -eq "'") { $quote = $character; $index++; continue }
        if ($character -eq '[' -or $character -eq '{' -or $character -eq '(') { $depth++ }
        elseif ($character -eq ']' -or $character -eq '}' -or $character -eq ')') { $depth-- }
        elseif ($character -eq ',' -and $depth -eq 0) {
            $parts.Add($Text.Substring($start, $index - $start))
            $start = $index + 1
        }
        $index++
    }
    if ($start -lt $Text.Length) { $parts.Add($Text.Substring($start)) }
    return @($parts)
}

function Get-TopLevelColon([string]$Text) {
    $depth = 0
    $quote = [char]0
    $index = 0
    while ($index -lt $Text.Length) {
        $character = $Text[$index]
        if ($quote -ne [char]0) {
            if ($character -eq '\') { $index += 2; continue }
            if ($character -eq $quote) { $quote = [char]0 }
            $index++
            continue
        }
        if ($character -eq '"' -or $character -eq "'") { $quote = $character; $index++; continue }
        if ($character -eq '[' -or $character -eq '{' -or $character -eq '(') { $depth++ }
        elseif ($character -eq ']' -or $character -eq '}' -or $character -eq ')') { $depth-- }
        elseif ($character -eq ':' -and $depth -eq 0) { return $index }
        $index++
    }
    return -1
}

function Get-ObjectFields([string]$Text) {
    # `{ label: "x", items: [...] }` as a table of raw value text.
    $fields = @{}
    foreach ($part in @(Split-TopLevel $Text.Substring(1, $Text.Length - 2))) {
        $trimmed = $part.Trim()
        if ($trimmed.Length -eq 0) { continue }
        $colon = Get-TopLevelColon $trimmed
        if ($colon -lt 0) {
            throw "site/astro.config.ts has a sidebar entry that is not a name and a value: $trimmed"
        }
        $name = $trimmed.Substring(0, $colon).Trim().Trim('"', "'")
        $fields[$name] = $trimmed.Substring($colon + 1).Trim()
    }
    return $fields
}

function Get-StringLiteral([string]$Text) {
    $trimmed = $Text.Trim()
    if ($trimmed.Length -lt 2) {
        throw "site/astro.config.ts has a sidebar value that is not a string: $Text"
    }
    $quote = $trimmed[0]
    if ($quote -ne '"' -and $quote -ne "'") {
        throw "site/astro.config.ts has a sidebar value that is not a string: $Text"
    }
    if ($trimmed[$trimmed.Length - 1] -ne $quote) {
        throw "site/astro.config.ts has an unterminated string: $Text"
    }
    $value = $trimmed.Substring(1, $trimmed.Length - 2)
    return ($value -replace '\\"', '"' -replace "\\'", "'" -replace '\\\\', '\')
}

function Get-ConstArrays([string]$Text) {
    $arrays = @{}
    $pattern = '(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(\[)'
    foreach ($match in [regex]::Matches($Text, $pattern)) {
        $arrays[$match.Groups[1].Value] = (Get-BracketSlice $Text $match.Groups[2].Index)['Text']
    }
    return $arrays
}

function Resolve-ArrayLiteral([string]$Expression, [hashtable]$ConstArrays) {
    $trimmed = $Expression.Trim().TrimEnd(';')
    if ($trimmed.StartsWith("[")) {
        return (Get-BracketSlice $trimmed 0)['Text']
    }
    if ($trimmed -match '^[A-Za-z_$][A-Za-z0-9_$]*$') {
        if (-not $ConstArrays.ContainsKey($trimmed)) {
            throw "site/astro.config.ts points at '$trimmed', which is not a list in that file"
        }
        return $ConstArrays[$trimmed]
    }
    throw "site/astro.config.ts has a sidebar list that cannot be read: $trimmed"
}

function Get-SidebarNodes([string]$ArrayText, [hashtable]$ConstArrays) {
    $nodes = New-Object System.Collections.Generic.List[hashtable]
    foreach ($part in @(Split-TopLevel $ArrayText.Substring(1, $ArrayText.Length - 2))) {
        $trimmed = $part.Trim()
        if ($trimmed.Length -eq 0) { continue }
        # A string is a `docs` collection entry id; an object is a row with a
        # label and either a `link`, a group `items` list, or both.
        $node = @{ Id = $null; Link = $null; Label = $null; Children = @() }
        if ($trimmed[0] -eq '{') {
            $fields = Get-ObjectFields $trimmed
            foreach ($name in $fields.Keys) {
                if ($name -eq 'label') { $node['Label'] = Get-StringLiteral $fields[$name] }
                elseif ($name -eq 'link') { $node['Link'] = Get-StringLiteral $fields[$name] }
                elseif ($name -eq 'id' -or $name -eq 'slug') { $node['Id'] = Get-StringLiteral $fields[$name] }
                elseif ($name -eq 'items') { $node['Children'] = @(Get-SidebarNodes (Resolve-ArrayLiteral $fields[$name] $ConstArrays) $ConstArrays) }
            }
        }
        elseif ($trimmed[0] -eq '"' -or $trimmed[0] -eq "'") {
            $node['Id'] = Get-StringLiteral $trimmed
        }
        else {
            throw "site/astro.config.ts has a sidebar entry that cannot be read: $trimmed"
        }
        $nodes.Add($node)
    }
    return @($nodes)
}

function Get-SidebarLeaves($Nodes, [string]$Group, $Leaves) {
    foreach ($node in $Nodes) {
        $label = $Group
        if ($null -ne $node['Label']) {
            $label = if ($Group) { "$Group / $($node['Label'])" } else { $node['Label'] }
        }
        if ($null -ne $node['Id'] -or $null -ne $node['Link']) {
            $Leaves.Add(@{ Id = $node['Id']; Link = $node['Link']; Group = $label })
        }
        if (@($node['Children']).Count -gt 0) {
            Get-SidebarLeaves $node['Children'] $label $Leaves
        }
    }
}

# ---------------------------------------------------------------------------
# The two trees, keyed the way a sidebar row names a page.
# ---------------------------------------------------------------------------

function Get-PageInventory([string]$Directory, [string]$RepositoryRelative) {
    # Ordinal keys, not PowerShell's case-insensitive default: the site is
    # published to Linux, where a row that spells a page `readme` does not reach
    # `README.md` and the build cannot resolve the entry either.
    $pages = New-Object 'System.Collections.Hashtable' ([System.StringComparer]::Ordinal)
    foreach ($file in (Get-ChildItem -LiteralPath $Directory -Recurse -File)) {
        # A page is `.mdx` here and `.md` is accepted as well, so the inventory
        # does not depend on which of the two a tree happens to use.
        if ($file.Extension -notin @(".md", ".mdx")) { continue }
        $relative = $file.FullName.Substring($Directory.Length).TrimStart('\', '/') -replace '\\', '/'
        # The key is the page's name inside its tree, which is what an entry id
        # and a `link` are written as once the mount prefix is off, and the
        # extension comes off whole whichever of the two it is.
        $key = $relative.Substring(0, $relative.Length - $file.Extension.Length)
        if ($pages.ContainsKey($key)) {
            throw "$RepositoryRelative holds two pages named $key"
        }
        $pages[$key] = "$RepositoryRelative/$relative"
    }
    return $pages
}

$english = Get-PageInventory $englishDirectory "site/src/content/docs"
$chinese = Get-PageInventory $chineseDirectory "site/src/content/zh-CN"
if ($english.Count -eq 0 -or $chinese.Count -eq 0) {
    throw "One of the two documentation trees holds no page"
}

function Resolve-PageKey([hashtable]$Pages, [string]$Key) {
    # The key a row names is a page's path inside its tree. Two rows name a page
    # without spelling its whole path out: a section's landing page is the
    # `index.mdx` of its directory, and a tree's root is its own `index.mdx`
    # (Nimbus serves `guides/index.mdx` at `/guides/` and `index.mdx` at `/`).
    # Returns $null when neither the key nor its index page is a page of this
    # tree.
    if ($Key -eq '') { $Key = 'index' }
    if ($Pages.ContainsKey($Key)) { return $Key }
    if ($Pages.ContainsKey("$Key/index")) { return "$Key/index" }
    return $null
}

# Read as UTF-8 rather than through the ANSI code page: the labels in this file
# are Chinese, and a runner whose code page is not UTF-8 would decode them into
# mojibake inside the messages this audit prints.
$config = Remove-ConfigComments ([System.IO.File]::ReadAllText($configPath))
$constArrays = Get-ConstArrays $config

$sidebarMatch = [regex]::Match($config, '(?<![A-Za-z0-9_$])sidebar\s*:\s*\{')
if (-not $sidebarMatch.Success) {
    throw "site/astro.config.ts has no sidebar object"
}
$sidebarObject = (Get-BracketSlice $config ($sidebarMatch.Index + $sidebarMatch.Length - 1))['Text']
$sidebarFields = Get-ObjectFields $sidebarObject
if (-not $sidebarFields.ContainsKey('items')) {
    throw "site/astro.config.ts has a sidebar with no items list"
}
$sidebarItems = Resolve-ArrayLiteral $sidebarFields['items'] $constArrays

$leaves = New-Object System.Collections.Generic.List[hashtable]
Get-SidebarLeaves (Get-SidebarNodes $sidebarItems $constArrays) "" $leaves
if ($leaves.Count -eq 0) {
    throw "site/astro.config.ts has a sidebar with no page in it"
}

# ---------------------------------------------------------------------------
# Both directions.
# ---------------------------------------------------------------------------

$failures = New-Object System.Collections.Generic.List[string]
$listed = New-Object 'System.Collections.Hashtable' ([System.StringComparer]::Ordinal)

function Add-Listing([hashtable]$Listed, [string]$Key) {
    if ($Listed.ContainsKey($Key)) {
        $Listed[$Key] = $Listed[$Key] + 1
        return
    }
    $Listed[$Key] = 1
}

foreach ($leaf in $leaves) {
    $where = if ($leaf['Group']) { "the sidebar group '$($leaf['Group'])'" } else { "the sidebar" }
    if ($null -ne $leaf['Id']) {
        # A string row is an entry id of the primary `docs` collection, which is
        # the tree served at the site root. A section's landing page is the entry
        # its directory is named after (`guides/index.mdx` is `guides`), so the id
        # answers to the page the same way a directory link does.
        $key = Resolve-PageKey $english $leaf['Id'].Trim().Trim('/')
        if ($null -eq $key) {
            $failures.Add("$where lists the entry id '$($leaf['Id'])', which is not a page in site/src/content/docs")
            continue
        }
        Add-Listing $listed "docs/$key"
        continue
    }

    $link = $leaf['Link'].Trim()
    $target = ($link -split '#')[0]
    $target = ($target -split '\?')[0]
    if ($target -match '^[A-Za-z][A-Za-z0-9+.-]*:' -or $target.StartsWith('//')) {
        $failures.Add("$where links to '$link', which leaves the site; a sidebar row is held to the site's own pages")
        continue
    }
    if (-not $target.StartsWith('/')) { $target = "/$target" }
    $path = $target.TrimEnd('/')
    if ($path.StartsWith('/zh-CN/')) {
        $key = Resolve-PageKey $chinese $path.Substring('/zh-CN/'.Length)
        if ($null -eq $key) {
            $failures.Add("$where links to '$link', which is not a page in site/src/content/zh-CN")
            continue
        }
        Add-Listing $listed "zh-CN/$key"
        continue
    }
    if ($path -eq '/zh-CN') {
        # The Chinese collection's own root is the zh-CN tree's index.mdx, which
        # is the landing page that tree is served at.
        if (-not $chinese.ContainsKey('index')) {
            $failures.Add("$where links to '$link', which is not a page in site/src/content/zh-CN")
            continue
        }
        Add-Listing $listed "zh-CN/index"
        continue
    }
    $key = Resolve-PageKey $english $path.Trim('/')
    if ($null -eq $key) {
        $failures.Add("$where links to '$link', which is not a page in site/src/content/docs")
        continue
    }
    Add-Listing $listed "docs/$key"
}

foreach ($key in @($english.Keys | Sort-Object)) {
    if (-not $listed.ContainsKey("docs/$key")) {
        $failures.Add("$($english[$key]) is not in the sidebar's English groups")
    }
}
foreach ($key in @($chinese.Keys | Sort-Object)) {
    if (-not $listed.ContainsKey("zh-CN/$key")) {
        $failures.Add("$($chinese[$key]) is not in the sidebar's Chinese groups")
    }
}
foreach ($entry in @($listed.GetEnumerator() | Sort-Object Name)) {
    if ($entry.Value -gt 1) {
        $parts = $entry.Name -split '/', 2
        $page = if ($parts[0] -eq 'zh-CN') { $chinese[$parts[1]] } else { $english[$parts[1]] }
        $failures.Add("$page is listed $($entry.Value) times in the sidebar")
    }
}

if ($failures.Count -gt 0) {
    # `-ErrorAction Continue` on each line: the preference above is Stop, which
    # would end the script at the first one and hide the rest of them.
    $failures | ForEach-Object { Write-Error $_ -ErrorAction Continue }
    throw "Sidebar audit failed: $($failures.Count) disagreement(s) between site/astro.config.ts and the two content trees"
}

$message = "Sidebar audit passed: {0} English page(s) and {1} Chinese page(s), every page listed once." -f $english.Count, $chinese.Count
Write-Output $message
