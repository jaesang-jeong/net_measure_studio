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
