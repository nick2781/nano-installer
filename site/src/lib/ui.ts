const english = {
  search: "Search documentation",
  searchShort: "Search",
  searchPlaceholder: "Search documentation...",
  searchEmpty: "Type to search",
  searching: "Searching...",
  noResults: "No results found",
  searchError: "Search is unavailable. Please try again.",
  navigation: "Navigation",
  openNavigation: "Open navigation",
  closeNavigation: "Close navigation",
  filter: "Filter navigation",
  onThisPage: "On this page",
  jumpToSection: "Jump to section",
  overview: "Overview",
  previous: "Previous",
  next: "Next",
  pagination: "Pagination",
  updated: "Updated",
  copyPage: "Copy page",
  copied: "Copied",
  copyError: "Couldn't copy",
  markdown: "View as Markdown",
  editPage: "Edit this page",
  backToTop: "Back to top",
};

const chinese: typeof english = {
  search: "搜索文档",
  searchShort: "搜索",
  searchPlaceholder: "搜索文档...",
  searchEmpty: "输入关键词搜索",
  searching: "正在搜索...",
  noResults: "没有找到结果",
  searchError: "暂时无法搜索，请重试。",
  navigation: "文档目录",
  openNavigation: "打开文档目录",
  closeNavigation: "关闭文档目录",
  filter: "筛选目录",
  onThisPage: "本页内容",
  jumpToSection: "跳转到章节",
  overview: "概览",
  previous: "上一页",
  next: "下一页",
  pagination: "翻页",
  updated: "更新于",
  copyPage: "复制全文",
  copied: "已复制",
  copyError: "复制失败",
  markdown: "查看 Markdown",
  editPage: "编辑此页",
  backToTop: "回到顶部",
};

export function uiForLanguage(lang: string) {
  return lang === "zh-CN" ? chinese : english;
}

export function uiForPath(pathname: string) {
  return uiForLanguage(pathname.split("/").includes("zh-CN") ? "zh-CN" : "en");
}
