import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',
  title: 'Rust 学习笔记',
  description: '按章节由浅入深地学 Rust，配套 rust-learn 练习工程',
  cleanUrls: true,

  // 部署在 GitHub Pages 的项目子路径下，必须和仓库名一致；
  // 换成用户名.github.io 这类用户主页仓库时要改成 '/'
  base: '/rust-learn/',

  themeConfig: {
    nav: [
      { text: '首页', link: '/' },
      { text: '学习路线', link: '/guide/' },
      {
        text: '章节',
        items: [
          { text: '第 1 章 · 打印输出', link: '/guide/rust01-print' },
          { text: '第 2 章 · 变量与基本类型', link: '/guide/rust02-variables' },
          { text: '第 3 章 · 函数与表达式', link: '/guide/rust03-functions' },
          { text: '第 4 章 · 控制流', link: '/guide/rust04-control-flow' },
          { text: '第 5 章 · 所有权与借用', link: '/guide/rust05-ownership' },
          { text: '第 6 章 · 结构体与枚举', link: '/guide/rust06-structs-enums' },
          { text: '第 7 章 · 常用集合', link: '/guide/rust07-collections' },
          { text: '第 8 章 · 错误处理', link: '/guide/rust08-errors' },
          { text: '第 9 章 · 泛型与 trait', link: '/guide/rust09-generics-traits' },
          { text: '第 10 章 · 生命周期', link: '/guide/rust10-lifetimes' },
          { text: '第 11 章 · 闭包与迭代器', link: '/guide/rust11-closures-iterators' },
          { text: '第 12 章 · 智能指针', link: '/guide/rust12-smart-pointers' },
          { text: '第 13 章 · 并发编程', link: '/guide/rust13-concurrency' },
          { text: '第 14 章 · 模块、包与测试', link: '/guide/rust14-modules-tests' },
          { text: '第 15 章 · 宏', link: '/guide/rust15-macros' },
          { text: '第 16 章 · async/await', link: '/guide/rust16-async' },
          { text: '第 17 章 · 日期与时间', link: '/guide/rust17-datetime' },
          { text: '第 18 章 · 文本处理与正则', link: '/guide/rust18-text-regex' },
          { text: '第 19 章 · 文件、路径与 IO', link: '/guide/rust19-files-io' },
          { text: '第 20 章 · 序列化与配置', link: '/guide/rust20-serde-config' },
          { text: '第 21 章 · 命令行工具', link: '/guide/rust21-cli' },
          { text: '第 22 章 · 网络与 HTTP', link: '/guide/rust22-network-http' },
        ],
      },
    ],

    sidebar: [
      {
        text: '开始',
        items: [{ text: '学习路线与章节规划', link: '/guide/' }],
      },
      {
        text: '基础篇',
        items: [
          { text: '第 1 章 · 打印输出', link: '/guide/rust01-print' },
          { text: '第 2 章 · 变量与基本类型', link: '/guide/rust02-variables' },
          { text: '第 3 章 · 函数与表达式', link: '/guide/rust03-functions' },
          { text: '第 4 章 · 控制流', link: '/guide/rust04-control-flow' },
          { text: '第 5 章 · 所有权与借用', link: '/guide/rust05-ownership' },
          { text: '第 6 章 · 结构体与枚举', link: '/guide/rust06-structs-enums' },
        ],
      },
      {
        text: '进阶篇',
        items: [
          { text: '第 7 章 · 常用集合', link: '/guide/rust07-collections' },
          { text: '第 8 章 · 错误处理', link: '/guide/rust08-errors' },
          { text: '第 9 章 · 泛型与 trait', link: '/guide/rust09-generics-traits' },
          { text: '第 10 章 · 生命周期', link: '/guide/rust10-lifetimes' },
          { text: '第 11 章 · 闭包与迭代器', link: '/guide/rust11-closures-iterators' },
          { text: '第 12 章 · 智能指针', link: '/guide/rust12-smart-pointers' },
        ],
      },
      {
        text: '实战篇',
        items: [
          { text: '第 13 章 · 并发编程', link: '/guide/rust13-concurrency' },
          { text: '第 14 章 · 模块、包与测试', link: '/guide/rust14-modules-tests' },
          { text: '第 15 章 · 宏', link: '/guide/rust15-macros' },
          { text: '第 16 章 · async/await', link: '/guide/rust16-async' },
        ],
      },
      {
        text: '标准库与生态',
        items: [
          { text: '第 17 章 · 日期与时间', link: '/guide/rust17-datetime' },
          { text: '第 18 章 · 文本处理与正则', link: '/guide/rust18-text-regex' },
          { text: '第 19 章 · 文件、路径与 IO', link: '/guide/rust19-files-io' },
          { text: '第 20 章 · 序列化与配置', link: '/guide/rust20-serde-config' },
          { text: '第 21 章 · 命令行工具', link: '/guide/rust21-cli' },
          { text: '第 22 章 · 网络与 HTTP', link: '/guide/rust22-network-http' },
        ],
      },
      {
        text: '工程与发布',
        items: [
          { text: '第 23 章 · 测试进阶与基准（规划中）' },
          { text: '第 24 章 · Cargo 深入与发布（规划中）' },
          { text: '第 25 章 · unsafe 与 FFI（规划中）' },
          { text: '第 26 章 · 综合实战项目（规划中）' },
        ],
      },
    ],

    outline: { level: [2, 3], label: '本页目录' },
    docFooter: { prev: '上一章', next: '下一章' },
    returnToTopLabel: '回到顶部',
    sidebarMenuLabel: '目录',
    darkModeSwitchLabel: '主题',

    search: {
      provider: 'local',
      options: {
        translations: {
          button: { buttonText: '搜索文档', buttonAriaLabel: '搜索文档' },
          modal: {
            noResultsText: '没有找到相关内容',
            resetButtonTitle: '清除查询条件',
            footer: {
              selectText: '选择',
              navigateText: '切换',
              closeText: '关闭',
            },
          },
        },
      },
    },
  },
})
