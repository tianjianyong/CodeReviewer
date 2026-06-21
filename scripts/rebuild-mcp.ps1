# 重建 MCP server 二进制
# 用法：修改 rules/core 后跑这个脚本，然后重启 opencode 加载新版
#
# 原理：opencode 长持有 MCP 进程（stdio 协议要求），导致 cargo 无法覆盖 exe。
# 这个脚本先 kill 旧进程，再编译，然后你需要重启 opencode 让它重新 spawn 新版。

Write-Host "Stopping running codereviewer-mcp process..." -ForegroundColor Cyan
$proc = Get-Process codereviewer-mcp -ErrorAction SilentlyContinue
if ($proc) {
    Stop-Process -Id $proc.Id -Force
    Write-Host "Stopped PID $($proc.Id)" -ForegroundColor Green
} else {
    Write-Host "No running process found" -ForegroundColor Gray
}

Write-Host "`nBuilding release binary..." -ForegroundColor Cyan
cargo build --release -p codereviewer-mcp
if ($LASTEXITCODE -ne 0) {
    Write-Host "`nBuild failed" -ForegroundColor Red
    exit 1
}

Write-Host "`nBuild succeeded" -ForegroundColor Green
Write-Host "Restart opencode to load the new MCP server" -ForegroundColor Yellow
