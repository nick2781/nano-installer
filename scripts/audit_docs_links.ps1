<#
    Holds every in-site link in the built site to what the site serves.

    Nimbus ships an authoring rule for this -- `nimbus/internal-link` is set to
    `error` in site/astro.config.ts -- and it never sees a page here. Every
    authoring rule is handed the list `findMdxFiles()` builds, which is
    `walkFilesSync(dir, { extensions: [".mdx"] })` in dist/cli/index.js; `lint`
    and `check` both call it, and nothing else runs them: the rules whose
    `kind` is `build` are the only ones `astro build` evaluates, and
    `nimbus/internal-link` is not one of them. So on a `.md` site the rule is
    configured, implemented, and pointed at nothing.
    scripts/audit_docs_sidebar.ps1 is the same gap's other half and needs no
    build; this half does, because a heading anchor only exists once the heading
    has been rendered.

    So this reads the build's own output, site/dist, which is what the Docs
    workflow uploads to Pages: every `<a href>` in every HTML page, and for the
    ones that stay on this site, the file it lands on and the id it points at.

    What counts as in-site: a root-relative path under the site's `base`, a
    relative path beside the page that carries it, and an absolute URL on the
    site's own origin under that base -- all three are checked the same way. A
    `mailto:`, `tel:`, `data:` or `javascript:` href, a protocol-relative `//`
    href and an http(s) URL on any other origin are left alone: they are not
    this site's to serve. A root-relative path that falls outside `base` is
    checked and fails, because it resolves to the account's origin rather than to
    this project's Pages site and can only 404.

    Resolution is the host's: `/X/` is `dist/X/index.html`, `/X` is that same
    page, `/X/y.md` is the file itself, and `/` is `dist/index.html`. A URL and
    an id are both decoded before they are compared, so a percent-encoded
    fragment matches the Chinese heading it names. Paths are compared
    case-sensitively -- the workflow publishes to Linux, where they are.

    site/dist is read, never written: run `pnpm build` in site/ first, or run the
    Docs workflow, which builds and then runs this before it uploads.

    Usage:
      .\scripts\audit_docs_links.ps1
      .\scripts\audit_docs_links.ps1 -RepositoryRoot C:\src\nano-installer
#>
param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$configPath = Join-Path $RepositoryRoot "site/astro.config.ts"
$distDirectory = Join-Path $RepositoryRoot "site/dist"

if (-not (Test-Path -LiteralPath $distDirectory -PathType Container)) {
    Write-Error "site/dist is not there, so there is nothing to audit: this reads the built site, and 'pnpm build' in site/ is what writes it." -ErrorAction Continue
    exit 1
}
if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
    throw "Not found: $configPath"
}

# ---------------------------------------------------------------------------
# What the site is: its own origin, and the base it is served under.
# ---------------------------------------------------------------------------

# Read as UTF-8 rather than through the ANSI code page, the same way every page
# below is: the config and the pages are both UTF-8 files.
$config = [System.IO.File]::ReadAllText($configPath)
$siteMatch = [regex]::Match($config, '(?m)^\s*site\s*:\s*"([^"]*)"')
if (-not $siteMatch.Success) { throw "site/astro.config.ts has no site URL" }
$baseMatch = [regex]::Match($config, '(?m)^\s*base\s*:\s*"([^"]*)"')
if (-not $baseMatch.Success) { throw "site/astro.config.ts has no base" }
$siteUrl = $siteMatch.Groups[1].Value.TrimEnd('/')
$siteRoot = "$siteUrl/"
$origin = ([uri]$siteUrl).GetLeftPart([System.UriPartial]::Authority)
$base = $baseMatch.Groups[1].Value.TrimEnd('/')

# ---------------------------------------------------------------------------
# The build's output, as the paths it is served at.
# ---------------------------------------------------------------------------

$served = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::Ordinal)
$pages = New-Object System.Collections.Generic.List[string]
foreach ($file in (Get-ChildItem -LiteralPath $distDirectory -Recurse -File)) {
    $relative = $file.FullName.Substring($distDirectory.Length).TrimStart('\', '/') -replace '\\', '/'
    [void]$served.Add($relative)
    if ($file.Extension -eq ".html") { $pages.Add($relative) }
}
if ($pages.Count -eq 0) {
    Write-Error "site/dist holds no HTML page, so there is nothing to audit: build the site in site/ first." -ErrorAction Continue
    exit 1
}

$anchorIds = @{}

function ConvertFrom-UrlEncoding([string]$Text) {
    if ($Text.IndexOf('%') -lt 0) { return $Text }
    try { return [System.Uri]::UnescapeDataString($Text) }
    catch { return $Text }
}

function ConvertFrom-HtmlText([string]$Text) {
    if ($Text.IndexOf('&') -lt 0) { return $Text }
    return [System.Net.WebUtility]::HtmlDecode($Text)
}

function Get-AnchorIds([string]$Relative) {
    if ($anchorIds.ContainsKey($Relative)) { return $anchorIds[$Relative] }
    $ids = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::Ordinal)
    $text = [System.IO.File]::ReadAllText((Join-Path $distDirectory $Relative))
    $pattern = '(?:\sid|\sname)\s*=\s*(?:"([^"]*)"|''([^'']*)''|([^\s>]+))'
    foreach ($match in [regex]::Matches($text, $pattern)) {
        $value = if ($match.Groups[1].Success) { $match.Groups[1].Value }
        elseif ($match.Groups[2].Success) { $match.Groups[2].Value }
        else { $match.Groups[3].Value }
        [void]$ids.Add((ConvertFrom-UrlEncoding (ConvertFrom-HtmlText $value)))
    }
    $anchorIds[$Relative] = $ids
    return $ids
}

# ---------------------------------------------------------------------------
# Every link, page by page.
# ---------------------------------------------------------------------------

$hrefPattern = '<a\b[^>]*?\shref\s*=\s*(?:"([^"]*)"|''([^'']*)'')'
$failures = New-Object System.Collections.Generic.List[string]
$linkCount = 0
$anchorCount = 0

foreach ($page in @($pages | Sort-Object)) {
    $urlPath = if ($page -eq 'index.html') { '/' } else { '/' + $page }
    if ($page.EndsWith('/index.html')) {
        $urlPath = '/' + $page.Substring(0, $page.Length - 'index.html'.Length)
    }
    # A placeholder origin: nothing here reads the host, only the path a relative
    # href resolves to beside the page that carries it.
    $sourceUrl = New-Object System.Uri("https://docs.invalid$base$urlPath")

    $text = [System.IO.File]::ReadAllText((Join-Path $distDirectory $page))
    foreach ($match in [regex]::Matches($text, $hrefPattern)) {
        $raw = if ($match.Groups[1].Success) { $match.Groups[1].Value } else { $match.Groups[2].Value }
        $href = (ConvertFrom-HtmlText $raw).Trim()
        if ($href.Length -eq 0) { continue }

        if ($href.StartsWith('//')) { continue }
        if ($href -match '^https?://') {
            # The site's own origin through its base is this site; anything else
            # is somebody else's page and not this audit's to hold. Only the
            # origin comes off, so what is left is still root-relative and still
            # carries the base the check below strips.
            if ($href -eq $siteUrl -or $href.StartsWith($siteRoot)) { $href = $href.Substring($origin.Length) }
            else { continue }
        }
        elseif ($href -match '^[A-Za-z][A-Za-z0-9+.-]*:') { continue }

        $linkCount++

        try {
            $target = New-Object System.Uri($sourceUrl, $href)
        }
        catch {
            $failures.Add("site/dist/$page -> '$raw': this cannot be read as a link to this site")
            continue
        }

        $anchor = ''
        if ($target.Fragment.Length -gt 1) {
            $anchor = ConvertFrom-UrlEncoding $target.Fragment.Substring(1)
            $anchorCount++
        }

        $targetPath = ConvertFrom-UrlEncoding $target.AbsolutePath
        if ($base -ne '' -and $targetPath -ne $base -and -not $targetPath.StartsWith("$base/")) {
            $failures.Add("site/dist/$page -> '$raw': '$targetPath' is outside the site's base '$base', where this project serves nothing")
            continue
        }
        $route = $targetPath.Substring($base.Length).TrimStart('/')

        $relative = if ($route -eq '') { 'index.html' }
        elseif ($route.EndsWith('/')) { "$route" + "index.html" }
        elseif ($served.Contains($route)) { $route }
        else { "$route/index.html" }

        if (-not $served.Contains($relative)) {
            $failures.Add("site/dist/$page -> '$raw': the target page site/dist/$relative does not exist")
            continue
        }
        if ($anchor.Length -eq 0 -or -not $relative.EndsWith('.html')) { continue }
        if (-not (Get-AnchorIds $relative).Contains($anchor)) {
            $failures.Add("site/dist/$page -> '$raw': site/dist/$relative has no id=""$anchor""")
        }
    }
}

if ($failures.Count -gt 0) {
    # `-ErrorAction Continue` on each line: the preference above is Stop, which
    # would end the script at the first one and hide the rest of them.
    $failures | ForEach-Object { Write-Error $_ -ErrorAction Continue }
    throw "Documentation link audit failed: $($failures.Count) link(s) in site/dist do not resolve"
}

$message = "Documentation link audit passed: {0} page(s), {1} in-site link(s), {2} anchor(s), all resolved." -f $pages.Count, $linkCount, $anchorCount
Write-Output $message
