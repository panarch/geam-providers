# Geam Providers

이 저장소는 기존 Gleam 패키지의 native 함수를 Geam의 공개 provider API로 구현합니다. 각 패키지는 독립적인 Cargo workspace member이며, 사용할 provider를 애플리케이션에서 직접 선택합니다.

| Gleam 패키지 | Rust provider | 검증한 버전 |
| --- | --- | --- |
| [`gleam_regexp`](https://hex.pm/packages/gleam_regexp) | [`geam-regexp`](gleam-regexp/README.md) | 1.1.1 |

Geam 의존성은 배포 버전이나 로컬 경로 대신 `main`의 커밋 `76c4ab7c6a2c0c35975bdd97e5de7c6284f895e7`에 고정했습니다. Gleam 패키지는 Hex 원본을 사용하며, 원본 소스를 수정하지 않습니다. provider crate의 게시와 게시물 소비 검증은 Geam의 해당 기능을 포함한 버전이 배포된 뒤 별도로 진행합니다.

테스트 순서와 커버리지 기준은 [테스트 지침](docs/development/testing.md), 코드 검토 기준은 [리뷰 정책](docs/development/review-policy.md)에 있습니다.
