<#
    Writes the three files that let an agent read this documentation site.

    The site is a docsify shell: the published HTML holds no prose at all, so a
    reader gets pages rendered in a browser and an agent that fetches the site
    root gets an empty page and stops. Every page is also published as the
    Markdown file it already is, so the way in is to say where those files are:

      docs/llms.txt      the index, one line per page, each pointing at the
                         Markdown file rather than at the viewer
      docs/llms-full.txt the English pages in one document, for a reader that
                         would rather fetch once
      docs/robots.txt    everything public, and where the sitemap is
      docs/sitemap.xml   the same page list as XML

    The words live in scripts/docs_index.json: a script file may only hold
    non-ASCII text when it carries a UTF-8 BOM, and a data file says what it is
    without that trap. Titles are not left free to drift -- a page that carries a
    Markdown heading must be named in the data file exactly as its heading reads,
    because the index is what an agent quotes back.

    The check is the same code that writes the files, so the two cannot disagree:
    a page with no entry, an entry with no page, an entry listed twice, an empty
    title or description, a heading that disagrees, a repository file that is not
    there, and a page that belongs to no group are all failures. Adding a page
    without describing it therefore fails in CI instead of quietly missing from
    the index. A page named as excluded from the one-document file has to exist
    too, so a typo there cannot silently drop a page from it.

    -Verify compares what is on disk with what would be written and fails on any
    difference. That is what the docs workflow runs before it publishes. The
    comparison is made on text with its line endings normalized, because a
    Windows checkout and a Linux runner hold the same file differently, and a
    BOM is refused outright because these files are published as they are.

    Usage:
      .\scripts\build_docs_index.ps1            # write the files
      .\scripts\build_docs_index.ps1 -Verify    # fail if they are out of date
#>
param(
    [switch]$Verify,
    [string]$DocumentationDirectory,
    [string]$DataFile
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$docsDirectory = if ($DocumentationDirectory) {
    $DocumentationDirectory
} else {
    Join-Path $repoRoot "docs"
}
$dataPath = if ($DataFile) {
    $DataFile
} else {
    Join-Path $PSScriptRoot "docs_index.json"
}

foreach ($required in @($docsDirectory, $dataPath)) {
    if (-not (Test-Path -LiteralPath $required)) {
        throw "Not found: $required"
    }
}

$data = Get-Content -Raw -Encoding UTF8 -LiteralPath $dataPath | ConvertFrom-Json
$site = [string]$data.site
if (-not $site.EndsWith("/")) {
    $site += "/"
}

$failures = @()

# Every page the documentation directory holds, by its path inside it. Files
# whose names begin with an underscore are the site's own navigation fragments
# rather than pages, and docsify never shows them on their own.
$onDisk = @{}
foreach ($file in Get-ChildItem -LiteralPath $docsDirectory -Recurse -Filter "*.md" -File) {
    if ($file.Name.StartsWith("_")) {
        continue
    }
    $relative = $file.FullName.Substring($docsDirectory.Length + 1).Replace("\", "/")
    $onDisk[$relative] = $file.FullName
}

function Get-MarkdownHeading([string]$FilePath) {
    foreach ($line in [System.IO.File]::ReadAllLines($FilePath)) {
        if ($line.StartsWith("# ")) {
            return $line.Substring(2).Trim()
        }
    }
    return $null
}

$entries = @()
$listed = @{}
foreach ($page in @($data.pages)) {
    $path = [string]$page.path
    $title = [string]$page.title
    $description = [string]$page.description

    if (-not $path) {
        $failures += "an entry in $dataPath names no path"
        continue
    }
    if (-not $title -or -not $description) {
        $failures += "$path has an empty title or description in $dataPath"
        continue
    }
    if ($listed.ContainsKey($path)) {
        $failures += "$path is listed twice in $dataPath"
        continue
    }
    $listed[$path] = $true

    if (-not $onDisk.ContainsKey($path)) {
        $failures += "$path is listed in $dataPath but is not a page under $docsDirectory"
        continue
    }

    # A page that carries a Markdown heading is the authority on its own name;
    # the data file supplies one only for the pages that begin with HTML.
    $heading = Get-MarkdownHeading $onDisk[$path]
    if ($heading -and $heading -ne $title) {
        $failures += "$path is headed ""$heading"" but $dataPath calls it ""$title"""
    }

    # Whether this page is in the one-document file is answered by the group it belongs to, and a
    # page may override that answer: the site's landing page is written in both languages, so it is
    # listed in the index but is not part of the English document.
    $override = $null
    $overrideProperty = $page.PSObject.Properties["inFull"]
    if ($null -ne $overrideProperty) {
        $override = [bool]$overrideProperty.Value
    }

    $entries += [pscustomobject]@{
        Path        = $path
        Title       = $title
        Description = $description
        InFull      = $override
    }
}
foreach ($path in $onDisk.Keys) {
    if (-not $listed.ContainsKey($path)) {
        $failures += "$path is a page under $docsDirectory but has no entry in $dataPath"
    }
}

# Each page belongs to the longest group whose prefix matches it. A page that
# matches none is a failure rather than a page that quietly vanishes from the
# index, and a group in the data file is what decides the order within it.
$owner = @{}
foreach ($entry in $entries) {
    $best = $null
    foreach ($candidate in @($data.groups)) {
        $prefix = [string]$candidate.prefix
        $matches = if ($prefix -eq "") {
            $entry.Path -notmatch "/"
        } else {
            $entry.Path.StartsWith($prefix)
        }
        if ($matches -and ($null -eq $best -or $prefix.Length -gt $best.Length)) {
            $best = $prefix
        }
    }
    if ($null -eq $best) {
        $failures += "$($entry.Path) belongs to no group in $dataPath"
        continue
    }
    $owner[$entry.Path] = $best
}

$groups = @()
foreach ($group in @($data.groups)) {
    $prefix = [string]$group.prefix
    $members = @($entries | Where-Object { $owner.ContainsKey($_.Path) -and $owner[$_.Path] -eq $prefix })
    $groups += [pscustomobject]@{
        Heading = [string]$group.heading
        Prefix  = $prefix
        InFull  = [bool]$group.inFull
        Members = $members
    }
}

# The files outside the published site that an agent working in this repository
# should be told about, checked so the list cannot rot.
$repository = $data.repository
$repositoryLines = @()
foreach ($file in @($repository.files)) {
    $path = [string]$file.path
    if (-not (Test-Path -LiteralPath (Join-Path $repoRoot $path) -PathType Leaf)) {
        $failures += "$path is named in $dataPath but is not in the repository"
        continue
    }
    $link = [string]$repository.base + $path
    $repositoryLines += "- [$([string]$file.title)]($link): $([string]$file.description)"
}

# A page named as excluded from the one-document file has to be a page, so a typo
# here cannot quietly drop a document from it. Which pages that file carries is
# the group's own answer, so a new page joins it because of where it lives rather
# than because someone remembered to list it.
$full = $data.full
$excluded = @{}
foreach ($path in @($full.exclude)) {
    $path = [string]$path
    if (-not $listed.ContainsKey($path)) {
        $failures += "$path is left out of llms-full.txt but is not a page listed in $dataPath"
        continue
    }
    $excluded[$path] = $true
}
$inFull = @{}
foreach ($group in $groups) {
    if (-not $group.InFull) {
        continue
    }
    foreach ($member in $group.Members) {
        $inFull[$member.Path] = $true
    }
}
foreach ($entry in $entries) {
    if ($null -eq $entry.InFull) {
        continue
    }
    if ($entry.InFull) {
        $inFull[$entry.Path] = $true
    }
    else {
        $inFull.Remove($entry.Path) | Out-Null
    }
}
if ($inFull.Count -eq 0) {
    $failures += "no page would be in llms-full.txt, so that file would be empty"
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    throw "The documentation index does not match the pages it describes"
}

# llms.txt: an H1, a blockquote summary, then one section per group, each line a
# link to the page's own file with a note after it.
$llms = [System.Collections.Generic.List[string]]::new()
$llms.Add("# $([string]$data.title)")
$llms.Add("")
$llms.Add("> $([string]$data.summary)")
$llms.Add("")
$llms.Add([string]$data.note)
$llms.Add("")
$llms.Add("$([string]$full.label): ${site}llms-full.txt")
foreach ($group in $groups) {
    if ($group.Members.Count -eq 0) {
        continue
    }
    $llms.Add("")
    $llms.Add("## $($group.Heading)")
    $llms.Add("")
    foreach ($member in $group.Members) {
        $llms.Add("- [$($member.Title)]($site$($member.Path)): $($member.Description)")
    }
}
if ($repositoryLines.Count -gt 0) {
    $llms.Add("")
    $llms.Add("## $([string]$repository.heading)")
    $llms.Add("")
    foreach ($line in $repositoryLines) {
        $llms.Add($line)
    }
}
$llmsText = ($llms -join "`n") + "`n"

# robots.txt: everything here is public and meant to be read.
$robotsText = (@(
        "# Every page of this site is a Markdown file published beside the HTML",
        "# viewer, and /llms.txt is the index written for agents.",
        "",
        "User-agent: *",
        "Allow: /",
        "",
        "Sitemap: ${site}sitemap.xml"
    ) -join "`n") + "`n"

# sitemap.xml: the site root and every page's own file. The pages a crawler can
# read are the Markdown files, because the HTML is one viewer over all of them
# and its addresses differ only in a fragment.
function ConvertTo-XmlText([string]$Value) {
    return $Value.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;").Replace('"', "&quot;")
}

$sitemap = [System.Collections.Generic.List[string]]::new()
$sitemap.Add('<?xml version="1.0" encoding="UTF-8"?>')
$sitemap.Add('<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">')
$sitemap.Add("  <url>")
$sitemap.Add("    <loc>$(ConvertTo-XmlText $site)</loc>")
$sitemap.Add("  </url>")
foreach ($entry in $entries) {
    $sitemap.Add("  <url>")
    $sitemap.Add("    <loc>$(ConvertTo-XmlText ($site + $entry.Path))</loc>")
    $sitemap.Add("  </url>")
}
$sitemap.Add("</urlset>")
$sitemapText = ($sitemap -join "`n") + "`n"

# llms-full.txt: the pages whose path is not in the excluded set, in the order the
# index lists them, each introduced by the address it came from so a reader can
# quote the one page rather than the one file.
$fullPages = @($entries | Where-Object { $inFull.ContainsKey($_.Path) -and -not $excluded.ContainsKey($_.Path) })
$fullLines = [System.Collections.Generic.List[string]]::new()
$fullLines.Add("# $([string]$full.title)")
$fullLines.Add("")
$fullLines.Add([string]$full.note)
foreach ($page in $fullPages) {
    $fullLines.Add("")
    $fullLines.Add("---")
    $fullLines.Add("")
    $fullLines.Add("Source: $site$($page.Path)")
    $fullLines.Add("")
    $content = [System.IO.File]::ReadAllText($onDisk[$page.Path], [System.Text.UTF8Encoding]::new($false))
    $fullLines.Add((($content -split "`r?`n") -join "`n").TrimEnd())
}
$fullText = ($fullLines -join "`n") + "`n"

$outputs = @(
    [pscustomobject]@{ Path = (Join-Path $docsDirectory "llms.txt"); Text = $llmsText },
    [pscustomobject]@{ Path = (Join-Path $docsDirectory "llms-full.txt"); Text = $fullText },
    [pscustomobject]@{ Path = (Join-Path $docsDirectory "robots.txt"); Text = $robotsText },
    [pscustomobject]@{ Path = (Join-Path $docsDirectory "sitemap.xml"); Text = $sitemapText }
)

$utf8 = [System.Text.UTF8Encoding]::new($false)
function Get-NormalizedText([string]$Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        throw "$Path carries a UTF-8 BOM; these files are published as they are, so it has to go"
    }
    $text = [System.IO.File]::ReadAllText($Path, $utf8)
    return ($text -split "`r?`n") -join "`n"
}

if ($Verify) {
    $stale = @()
    foreach ($output in $outputs) {
        if (-not (Test-Path -LiteralPath $output.Path -PathType Leaf)) {
            $stale += "$($output.Path) is not there"
            continue
        }
        if ((Get-NormalizedText $output.Path) -ne (($output.Text -split "`r?`n") -join "`n")) {
            $stale += "$($output.Path) does not hold what the pages say"
        }
    }
    if ($stale.Count -gt 0) {
        $stale | ForEach-Object { Write-Error $_ }
        throw "The documentation index is out of date; run .\scripts\build_docs_index.ps1"
    }
    Write-Output "Documentation index verified: $($entries.Count) page(s) and $(@($repositoryLines).Count) repository file(s)"
}
else {
    foreach ($output in $outputs) {
        [System.IO.File]::WriteAllText($output.Path, $output.Text, $utf8)
    }
    Write-Output "Documentation index written: $($entries.Count) page(s) into $($outputs.Count) file(s)"
    foreach ($output in $outputs) {
        $relative = $output.Path.Substring($repoRoot.Length + 1).Replace("\", "/")
        Write-Output "  $relative"
    }
}
