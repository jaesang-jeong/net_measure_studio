# Log — <프로젝트명>

> append-only. 기존 줄은 수정·삭제하지 않는다. 새 항목은 항상 파일 끝에.
> 형식: `## YYYY-MM-DD HH:MM — <작업 제목>` + [DONE] 블록

---


## 2026-11-09 18:03 — GitHub 리포지토리 푸시 준비 및 전체 프로젝트 커밋
[DONE]
1. 수정 파일:
   - .gitignore
2. 검증:
   - git commit 완료 (커밋 해시: 8dba58c)
   - git push -u origin main 시도 → remote: Repository not found (GitHub 웹에서 빈 저장소 생성 필요)
3. 미해결 / 제안:
   - GitHub 웹(https://github.com/new)에서 저장소 `net_measure_studio` 생성 후 `push_to_github.bat` 실행 필요


## 2026-11-09 18:24 — GitHub 리포지토리 푸시 시도 (403 권한 확인)
[DONE]
1. 수정 파일:
   - 없음
2. 검증:
   - git push -u origin main 실행 →
     remote: Permission to jaesang-jeong/net_measure_studio.git denied to jaesang-jeong.
     fatal: The requested URL returned error: 403
3. 미해결 / 제안:
   - 원격 저장소(`net_measure_studio`)는 정상 인식되었으나, 로컬 PC에 캐시된 GitHub 자격 증명 권한 만료로 403 발생.
   - push_to_github.bat 직접 실행 또는 GitHub Personal Access Token(PAT)을 통한 인증 갱신 제안.


## 2026-11-09 18:27 — push_to_github.bat 디렉터리 경로 및 구문 오류 수정
[DONE]
1. 수정 파일:
   - push_to_github.bat
2. 검증:
   - cd /d "%~dp0" 및 GIT_DISCOVERY_ACROSS_FILESYSTEM=1 추가로 fatal: not a git repository 오류 해결
   - 배치 괄호 구문 분기 오류 수정
3. 미해결 / 제안:
   - 수정된 push_to_github.bat 재실행 안내


## 2026-11-09 18:28 — 만료된 Windows GitHub 자격 증명 캐시 삭제
[DONE]
1. 수정 파일:
   - 없음
2. 검증:
   - cmdkey /delete:git:https://github.com && cmdkey /delete:git:https://jaesang-jeong@github.com → 삭제 성공
3. 미해결 / 제안:
   - push_to_github.bat 실행 시 브라우저 로그인 창을 통한 최신 인증 안내


## 2026-11-09 18:37 — Cygwin Git 충돌 해결 및 공식 Windows Git 경로 우선 적용
[DONE]
1. 수정 파일:
   - push_to_github.bat
2. 검증:
   - where git 확인 시 C:\cygwin64\bin\git.exe 가 1순위로 잡혀 credential-manager 가 동작하지 않던 문제 발견
   - C:\Program Files\Git\cmd\git.exe 우선 지정 로직 추가하여 정상 GCM 브라우저 인증 환경 구성
3. 미해결 / 제안:
   - 수정된 push_to_github.bat 재실행 안내
