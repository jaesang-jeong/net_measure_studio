# UTF-8 인코딩 설정
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

Set-Location -LiteralPath $PSScriptRoot

# Git 실행 파일 경로 탐색 (Windows 공식 Git 우선)
$gitExe = "git"
if (Test-Path "C:\Program Files\Git\cmd\git.exe") {
    $gitExe = "C:\Program Files\Git\cmd\git.exe"
} elseif (Test-Path "C:\Program Files (x86)\Git\cmd\git.exe") {
    $gitExe = "C:\Program Files (x86)\Git\cmd\git.exe"
}

$env:GIT_DISCOVERY_ACROSS_FILESYSTEM = "1"

function Show-Menu {
    Clear-Host
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host "  Network Measurement Studio - GitHub 푸시 도우미" -ForegroundColor Cyan
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host "  저장소 위치 : $PSScriptRoot"
    Write-Host "  Git 프로그램: $gitExe"
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  [1] GitHub 토큰(PAT)으로 100% 확실하게 푸시하기 (가장 추천!)" -ForegroundColor Yellow
    Write-Host "  [2] 웹 브라우저에서 새 토큰(PAT) 발급 페이지 바로 열기" -ForegroundColor Green
    Write-Host "  [3] 기본 푸시 시도 (기존 자격 증명 / 브라우저 로그인)"
    Write-Host "  [4] 종료"
    Write-Host ""
    Write-Host "========================================================" -ForegroundColor Cyan
}

while ($true) {
    Show-Menu
    $choice = Read-Host "작업 번호를 선택하세요 (1-4) [기본값: 1]"
    if ($choice -eq $null) { exit 0 }
    if ([string]::IsNullOrWhiteSpace($choice)) { $choice = "1" }

    switch ($choice) {
        "1" {
            Write-Host "`n--------------------------------------------------------" -ForegroundColor Cyan
            Write-Host "  [1] GitHub Personal Access Token (PAT) 푸시 모드" -ForegroundColor Cyan
            Write-Host "--------------------------------------------------------" -ForegroundColor Cyan
            
            $user = Read-Host "GitHub 계정 아이디 [기본값: jaesang-jeong]"
            if ([string]::IsNullOrWhiteSpace($user)) { $user = "jaesang-jeong" }
            
            $repo = Read-Host "GitHub 저장소 이름 [기본값: net_measure_studio]"
            if ([string]::IsNullOrWhiteSpace($repo)) { $repo = "net_measure_studio" }

            Write-Host "`n* 토큰이 아직 없으시면 메뉴 [2]번을 눌러 브라우저에서 발급받으실 수 있습니다." -ForegroundColor DarkGray
            $token = Read-Host "GitHub 토큰(PAT)을 붙여넣기 하세요 (ghp_...)"
            
            if ([string]::IsNullOrWhiteSpace($token)) {
                Write-Host "`n[경고] 토큰이 입력되지 않았습니다. 메인 메뉴로 돌아갑니다." -ForegroundColor Red
                Start-Sleep -Seconds 2
                continue
            }

            $token = $token.Trim()
            Write-Host "`n[INFO] GitHub로 푸시를 진행합니다..." -ForegroundColor Green
            $targetUrl = "https://${token}@github.com/${user}/${repo}.git"
            
            & $gitExe push $targetUrl main
            if ($LASTEXITCODE -eq 0) {
                Write-Host "`n========================================================" -ForegroundColor Green
                Write-Host "  [SUCCESS] GitHub 푸시가 완벽하게 성공했습니다!" -ForegroundColor Green
                Write-Host "  확인 URL: https://github.com/${user}/${repo}" -ForegroundColor Green
                Write-Host "========================================================" -ForegroundColor Green
                # 보안을 위해 origin URL 복원 (토큰 노출 방지)
                & $gitExe remote set-url origin "https://github.com/${user}/${repo}.git"
                Read-Host "`n완료되었습니다. 엔터 키를 누르면 종료합니다."
                exit 0
            } else {
                Write-Host "`n========================================================" -ForegroundColor Red
                Write-Host "  [ERROR] 푸시 실패!" -ForegroundColor Red
                Write-Host "  1. 토큰에 'repo' 권한이 체크되어 있는지 확인해 주세요."
                Write-Host "  2. https://github.com/${user}/${repo} 저장소가 실제 생성되어 있는지 확인해 주세요."
                Write-Host "========================================================" -ForegroundColor Red
                Read-Host "`n엔터 키를 누르면 메뉴로 돌아갑니다."
            }
        }
        "2" {
            Write-Host "`n[안내] 웹 브라우저에서 토큰 생성 페이지를 엽니다..." -ForegroundColor Green
            Write-Host "- 'repo' 권한이 자동 체크되어 있습니다."
            Write-Host "- 페이지 맨 아래 초록색 [Generate token] 버튼을 누른 후 생성된 토큰(ghp_...)을 복사하세요.`n"
            Start-Process "https://github.com/settings/tokens/new?scopes=repo&description=net_measure_studio"
            Read-Host "토큰을 복사하셨으면 엔터 키를 눌러 메뉴로 이동하세요."
        }
        "3" {
            Write-Host "`n[INFO] 기본 푸시 (git push -u origin main) 시도 중..." -ForegroundColor Green
            & $gitExe push -u origin main
            if ($LASTEXITCODE -eq 0) {
                Write-Host "`n[SUCCESS] 푸시 성공!" -ForegroundColor Green
                Read-Host "`n완료되었습니다. 엔터 키를 누르면 종료합니다."
                exit 0
            } else {
                Write-Host "`n[ERROR] 푸시 실패 (403 Forbidden 등) - [1]번 토큰 푸시를 이용해 주세요." -ForegroundColor Red
                Read-Host "`n엔터 키를 누르면 메뉴로 돌아갑니다."
            }
        }
        "4" {
            exit 0
        }
    }
}
