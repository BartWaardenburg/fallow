use criterion::{Criterion, criterion_group, criterion_main};

use fallow_config::{DetectConfig, FallowConfig, OutputFormat};

fn create_test_config(root: std::path::PathBuf) -> fallow_config::ResolvedConfig {
    FallowConfig {
        entry: vec![],
        ignore: vec![],
        detect: DetectConfig::default(),
        frameworks: None,
        framework: vec![],
        workspaces: None,
        ignore_dependencies: vec![],
        ignore_exports: vec![],
        output: OutputFormat::Human,
    }
    .resolve(root, 4, true)
}

fn bench_parse_file(c: &mut Criterion) {
    // Create a temporary file with typical TypeScript content
    let temp_dir = std::env::temp_dir().join("fallow-bench");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("bench.ts");
    std::fs::write(
        &test_file,
        r#"
import { useState, useEffect, useCallback, useMemo } from 'react';
import type { FC, ReactNode, MouseEvent } from 'react';
import * as lodash from 'lodash';
import axios from 'axios';

export interface Props {
    name: string;
    age: number;
    children?: ReactNode;
}

export type Status = 'active' | 'inactive' | 'pending';

export enum Color {
    Red = 'red',
    Green = 'green',
    Blue = 'blue',
}

export class UserService {
    private baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl;
    }

    async getUser(id: number) {
        return axios.get(`${this.baseUrl}/users/${id}`);
    }

    async listUsers() {
        return axios.get(`${this.baseUrl}/users`);
    }
}

export const useUser = (id: number) => {
    const [user, setUser] = useState(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const service = new UserService('/api');
        service.getUser(id).then(res => {
            setUser(res.data);
            setLoading(false);
        });
    }, [id]);

    return { user, loading };
};

export const formatName = (first: string, last: string): string => {
    return `${first} ${last}`;
};

export const capitalize = (s: string): string => {
    return s.charAt(0).toUpperCase() + s.slice(1);
};

export default function App({ name, age }: Props) {
    const { user, loading } = useUser(1);
    const fullName = useMemo(() => formatName(name, 'Doe'), [name]);

    const handleClick = useCallback((e: MouseEvent) => {
        console.log(e);
    }, []);

    if (loading) return null;

    return null;
}
"#,
    )
    .unwrap();

    let file = fallow_core::discover::DiscoveredFile {
        id: fallow_core::discover::FileId(0),
        path: test_file.clone(),
        size_bytes: std::fs::metadata(&test_file).unwrap().len(),
    };

    c.bench_function("parse_single_file", |b| {
        b.iter(|| {
            fallow_core::extract::parse_single_file(&file);
        });
    });

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn bench_full_pipeline(c: &mut Criterion) {
    // Create a small test project
    let temp_dir = std::env::temp_dir().join("fallow-bench-project");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("src")).unwrap();

    // Create package.json
    std::fs::write(
        temp_dir.join("package.json"),
        r#"{"name": "bench-project", "main": "src/index.ts", "dependencies": {"react": "^18"}}"#,
    )
    .unwrap();

    // Create 10 source files
    for i in 0..10 {
        let content = format!(
            r#"
export const value{i} = {i};
export function fn{i}() {{ return {i}; }}
export type Type{i} = {{ value: number }};
"#
        );
        std::fs::write(temp_dir.join(format!("src/module{i}.ts")), content).unwrap();
    }

    // Create index that imports some
    let imports: Vec<String> = (0..5)
        .map(|i| format!("import {{ value{i} }} from './module{i}';"))
        .collect();
    let uses: Vec<String> = (0..5).map(|i| format!("console.log(value{i});")).collect();
    std::fs::write(
        temp_dir.join("src/index.ts"),
        format!("{}\n{}\n", imports.join("\n"), uses.join("\n")),
    )
    .unwrap();

    let config = create_test_config(temp_dir.clone());

    c.bench_function("full_pipeline_10_files", |b| {
        b.iter(|| {
            let _ = fallow_core::analyze(&config);
        });
    });

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

criterion_group!(benches, bench_parse_file, bench_full_pipeline);
criterion_main!(benches);
