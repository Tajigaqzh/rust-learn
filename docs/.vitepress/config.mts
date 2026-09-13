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
          { text: '第 10 章 · 生命周期（规划中）' },
          { text: '第 11 章 · 闭包与迭代器（规划中）' },
          { text: '第 12 章 · 智能指针（规划中）' },
        ],
      },
      {
        text: '实战篇',
        items: [
          { text: '第 13 章 · 并发编程（规划中）' },
          { text: '第 14 章 · 模块、包与测试（规划中）' },
          { text: '第 15 章 · 宏（规划中）' },
          { text: '第 16 章 · async/await（规划中）' },
        ],
      },
      {
        text: '标准库与生态',
        items: [
          { text: '第 17 章 · 日期与时间（规划中）' },
          { text: '第 18 章 · 文本处理与正则（规划中）' },
          { text: '第 19 章 · 文件、路径与 IO（规划中）' },
          { text: '第 20 章 · 序列化与配置（规划中）' },
          { text: '第 21 章 · 命令行工具（规划中）' },
          { text: '第 22 章 · 网络与 HTTP（规划中）' },
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
