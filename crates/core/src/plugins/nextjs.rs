//! Next.js framework plugin.
//!
//! Detects Next.js projects and marks App Router/Pages Router convention files,
//! middleware, instrumentation, and metadata files as entry points.
//! Parses next.config to extract pageExtensions and referenced dependencies.

use std::path::Path;

use super::config_parser;
use super::{Plugin, PluginResult};

pub struct NextJsPlugin;

const ENABLERS: &[&str] = &["next"];

const ENTRY_PATTERNS: &[&str] = &[
    // App Router convention files
    "app/**/page.{ts,tsx,js,jsx}",
    "app/**/layout.{ts,tsx,js,jsx}",
    "app/**/loading.{ts,tsx,js,jsx}",
    "app/**/error.{ts,tsx,js,jsx}",
    "app/**/not-found.{ts,tsx,js,jsx}",
    "app/**/template.{ts,tsx,js,jsx}",
    "app/**/default.{ts,tsx,js,jsx}",
    "app/**/route.{ts,tsx,js,jsx}",
    "app/**/global-error.{ts,tsx,js,jsx}",
    // App Router metadata files
    "app/**/opengraph-image.{ts,tsx,js,jsx}",
    "app/**/twitter-image.{ts,tsx,js,jsx}",
    "app/**/icon.{ts,tsx,js,jsx}",
    "app/**/apple-icon.{ts,tsx,js,jsx}",
    "app/**/manifest.{ts,tsx,js,jsx}",
    "app/**/sitemap.{ts,tsx,js,jsx}",
    "app/**/robots.{ts,tsx,js,jsx}",
    // Pages Router
    "pages/**/*.{ts,tsx,js,jsx}",
    // src/ variants of App Router convention files
    "src/app/**/page.{ts,tsx,js,jsx}",
    "src/app/**/layout.{ts,tsx,js,jsx}",
    "src/app/**/loading.{ts,tsx,js,jsx}",
    "src/app/**/error.{ts,tsx,js,jsx}",
    "src/app/**/not-found.{ts,tsx,js,jsx}",
    "src/app/**/template.{ts,tsx,js,jsx}",
    "src/app/**/default.{ts,tsx,js,jsx}",
    "src/app/**/route.{ts,tsx,js,jsx}",
    "src/app/**/global-error.{ts,tsx,js,jsx}",
    // src/ variants of App Router metadata files
    "src/app/**/opengraph-image.{ts,tsx,js,jsx}",
    "src/app/**/twitter-image.{ts,tsx,js,jsx}",
    "src/app/**/icon.{ts,tsx,js,jsx}",
    "src/app/**/apple-icon.{ts,tsx,js,jsx}",
    "src/app/**/manifest.{ts,tsx,js,jsx}",
    "src/app/**/sitemap.{ts,tsx,js,jsx}",
    "src/app/**/robots.{ts,tsx,js,jsx}",
    // src/ Pages Router
    "src/pages/**/*.{ts,tsx,js,jsx}",
    // Middleware and proxy
    "middleware.{ts,js}",
    "src/middleware.{ts,js}",
    "proxy.{ts,js}",
    "src/proxy.{ts,js}",
    // Instrumentation (Next.js 14+)
    "instrumentation.{ts,js}",
    "instrumentation-client.{ts,js}",
    "src/instrumentation.{ts,js}",
    "src/instrumentation-client.{ts,js}",
];

const CONFIG_PATTERNS: &[&str] = &["next.config.{ts,js,mjs,cjs}"];

const ALWAYS_USED: &[&str] = &[
    "next.config.{ts,js,mjs,cjs}",
    "next-env.d.ts",
    "favicon.ico",
    "src/i18n/request.{ts,js}",
    "src/i18n/routing.{ts,js}",
    "i18n/request.{ts,js}",
    "i18n/routing.{ts,js}",
];

const TOOLING_DEPENDENCIES: &[&str] = &[
    "next",
    "@next/font",
    "@next/mdx",
    "@next/bundle-analyzer",
    "@next/env",
];

// Used exports for App Router page files
const PAGE_EXPORTS: &[&str] = &["default"];
const LAYOUT_EXPORTS: &[&str] = &[
    "default",
    "metadata",
    "generateMetadata",
    "generateStaticParams",
];
const ROUTE_EXPORTS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const PAGES_ROUTER_EXPORTS: &[&str] = &[
    "default",
    "getStaticProps",
    "getStaticPaths",
    "getServerSideProps",
];
const ICON_EXPORTS: &[&str] = &["default", "size", "contentType", "generateImageMetadata"];
const OG_IMAGE_EXPORTS: &[&str] = &[
    "default",
    "size",
    "contentType",
    "generateImageMetadata",
    "alt",
];
const MANIFEST_EXPORTS: &[&str] = &["default"];
const SITEMAP_EXPORTS: &[&str] = &["default", "generateSitemaps"];
const ROBOTS_EXPORTS: &[&str] = &["default"];

impl Plugin for NextJsPlugin {
    fn name(&self) -> &'static str {
        "nextjs"
    }

    fn enablers(&self) -> &'static [&'static str] {
        ENABLERS
    }

    fn entry_patterns(&self) -> &'static [&'static str] {
        ENTRY_PATTERNS
    }

    fn config_patterns(&self) -> &'static [&'static str] {
        CONFIG_PATTERNS
    }

    fn always_used(&self) -> &'static [&'static str] {
        ALWAYS_USED
    }

    fn tooling_dependencies(&self) -> &'static [&'static str] {
        TOOLING_DEPENDENCIES
    }

    fn used_exports(&self) -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            // App Router pages
            ("app/**/page.{ts,tsx,js,jsx}", PAGE_EXPORTS),
            ("app/**/layout.{ts,tsx,js,jsx}", LAYOUT_EXPORTS),
            ("app/**/route.{ts,tsx,js,jsx}", ROUTE_EXPORTS),
            // Pages Router
            ("pages/**/*.{ts,tsx,js,jsx}", PAGES_ROUTER_EXPORTS),
            // src/ variants
            ("src/app/**/page.{ts,tsx,js,jsx}", PAGE_EXPORTS),
            ("src/app/**/layout.{ts,tsx,js,jsx}", LAYOUT_EXPORTS),
            ("src/app/**/route.{ts,tsx,js,jsx}", ROUTE_EXPORTS),
            ("src/pages/**/*.{ts,tsx,js,jsx}", PAGES_ROUTER_EXPORTS),
            // Metadata image files
            ("app/**/icon.{ts,tsx,js,jsx}", ICON_EXPORTS),
            ("app/**/apple-icon.{ts,tsx,js,jsx}", ICON_EXPORTS),
            ("app/**/opengraph-image.{ts,tsx,js,jsx}", OG_IMAGE_EXPORTS),
            ("app/**/twitter-image.{ts,tsx,js,jsx}", OG_IMAGE_EXPORTS),
            // Metadata data files
            ("app/**/manifest.{ts,tsx,js,jsx}", MANIFEST_EXPORTS),
            ("app/**/sitemap.{ts,tsx,js,jsx}", SITEMAP_EXPORTS),
            ("app/**/robots.{ts,tsx,js,jsx}", ROBOTS_EXPORTS),
            // src/ variants of metadata image files
            ("src/app/**/icon.{ts,tsx,js,jsx}", ICON_EXPORTS),
            ("src/app/**/apple-icon.{ts,tsx,js,jsx}", ICON_EXPORTS),
            (
                "src/app/**/opengraph-image.{ts,tsx,js,jsx}",
                OG_IMAGE_EXPORTS,
            ),
            ("src/app/**/twitter-image.{ts,tsx,js,jsx}", OG_IMAGE_EXPORTS),
            // src/ variants of metadata data files
            ("src/app/**/manifest.{ts,tsx,js,jsx}", MANIFEST_EXPORTS),
            ("src/app/**/sitemap.{ts,tsx,js,jsx}", SITEMAP_EXPORTS),
            ("src/app/**/robots.{ts,tsx,js,jsx}", ROBOTS_EXPORTS),
        ]
    }

    fn resolve_config(&self, config_path: &Path, source: &str, _root: &Path) -> PluginResult {
        let mut result = PluginResult::default();

        // Extract import sources as referenced dependencies
        let imports = config_parser::extract_imports(source, config_path);
        for imp in &imports {
            let dep = crate::resolve::extract_package_name(imp);
            result.referenced_dependencies.push(dep);
        }

        // pageExtensions → modify entry patterns
        let page_extensions =
            config_parser::extract_config_string_array(source, config_path, &["pageExtensions"]);
        if !page_extensions.is_empty() {
            let ext_str = page_extensions.join(",");
            // Generate entry patterns with custom extensions
            let base_patterns = [
                "app/**/page",
                "app/**/layout",
                "app/**/loading",
                "app/**/error",
                "app/**/not-found",
                "app/**/template",
                "app/**/default",
                "app/**/route",
                "app/**/global-error",
                "pages/**/*",
                "src/app/**/page",
                "src/app/**/layout",
                "src/app/**/loading",
                "src/app/**/error",
                "src/app/**/not-found",
                "src/app/**/template",
                "src/app/**/default",
                "src/app/**/route",
                "src/app/**/global-error",
                "src/pages/**/*",
            ];
            for base in &base_patterns {
                result.entry_patterns.push(format!("{base}.{{{ext_str}}}"));
            }
        }

        result
    }
}
