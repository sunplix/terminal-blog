use crate::vfs::model::VfsError;

pub struct PathNormalizer;

impl PathNormalizer {
    /// 将用户输入的 raw 路径 (相对/绝对) 解析为规范的 "/a/b/c" 格式（无末尾斜杠，除根目录外）
    pub fn normalize(raw: &str, cwd: &str) -> Result<String, VfsError> {
        // 全部替换为正斜杠
        let raw = raw.replace('\\', "/");
        let cwd = cwd.replace('\\', "/");

        // 分割路径组件
        let mut comps: Vec<&str> = Vec::new();
        if raw.starts_with('/') {
            // 绝对路径：从根开始
            for comp in raw.split('/') {
                if comp.is_empty() || comp == "." {
                    continue;
                }
                if comp == ".." {
                    comps.pop();
                } else {
                    comps.push(comp);
                }
            }
        } else {
            // 相对路径：先把 cwd 的有效部分压进去
            for comp in cwd.split('/') {
                if comp.is_empty() || comp == "." {
                    continue;
                }
                if comp == ".." {
                    comps.pop();
                } else {
                    comps.push(comp);
                }
            }
            // 再处理 raw 自身
            for comp in raw.split('/') {
                if comp.is_empty() || comp == "." {
                    continue;
                }
                if comp == ".." {
                    comps.pop();
                } else {
                    comps.push(comp);
                }
            }
        }

        // 拼回字符串
        let result = if comps.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", comps.join("/"))
        };
        Ok(result)
    }

    /// 获取父目录路径，同样不带末尾斜杠（根目录的父仍是 None）
    pub fn parent(path: &str) -> Option<String> {
        let path = path.replace('\\', "/");
        let p = path.trim_end_matches('/');
        // 根或空就没有父
        if p.is_empty() || p == "/" {
            return None;
        }
        // 找最后一个 '/'
        if let Some(idx) = p.rfind('/') {
            let parent = &p[..idx];
            if parent.is_empty() {
                Some("/".to_string())
            } else {
                Some(parent.to_string())
            }
        } else {
            // 没有 slash，说明是相对名字，父就是 cwd 传进来的根？
            Some("/".to_string())
        }
    }

    /// basename，不会带斜杠
    pub fn basename(path: &str) -> Option<String> {
        let path = path.replace('\\', "/");
        path.rsplit('/').next().map(|s| s.to_string())
    }
}
