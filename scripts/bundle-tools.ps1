# 打包依赖工具到 resources/，供 tauri.conf.json bundle.resources 引用。
# 目前包含：企业微信官方 CLI wecom-cli（原生 win32-x64 exe，无需 Node 运行时）。
# 幂等：已存在则跳过；-Force 强制重新拉取。
param([switch]$Force)

$ErrorActionPreference = 'Stop'

$WecomCliVersion = '1.3.2'

$root = Split-Path -Parent $PSScriptRoot
$destDir = Join-Path $root 'resources\tools'
$destExe = Join-Path $destDir 'wecom-cli.exe'

function Invoke-WecomStage {
    $stage = Join-Path $env:TEMP ("ace-tools-stage-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    try {
        Write-Host "[bundle-tools] 安装 @wecom/cli@$WecomCliVersion 到临时目录..."
        npm install --prefix $stage "@wecom/cli@$WecomCliVersion" --no-audit --no-fund
        if ($LASTEXITCODE -ne 0) { throw "npm install 失败 (exit $LASTEXITCODE)" }

        $srcExe = Join-Path $stage 'node_modules\@wecom\cli-win32-x64\bin\wecom-cli.exe'
        if (-not (Test-Path $srcExe)) { throw "未找到平台二进制: $srcExe" }

        New-Item -ItemType Directory -Force -Path $destDir | Out-Null
        Copy-Item -Force $srcExe $destExe
        Set-Content -Path (Join-Path $destDir 'VERSION.txt') -Value "wecom-cli $WecomCliVersion (bundled native win32-x64)" -Encoding ascii
        Write-Host "[bundle-tools] 已生成 $destExe"
    }
    finally {
        Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
    }
}

if ((Test-Path $destExe) -and -not $Force) {
    Write-Host "[bundle-tools] 已存在，跳过: $destExe (使用 -Force 重新拉取)"
}
else {
    Invoke-WecomStage
}

# 冒烟验证：确认 exe 可执行且版本符合预期
$actual = & $destExe --version 2>&1 | Out-String
$actual = $actual.Trim()
Write-Host "[bundle-tools] 验证: $actual"
if ($actual -notmatch [regex]::Escape("wecom-cli $WecomCliVersion")) {
    Write-Warning "版本校验不匹配（预期 $WecomCliVersion）: $actual"
}
