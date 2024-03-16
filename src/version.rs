use std::cmp::Ordering;

#[derive(Debug)]
pub struct ModuleVersion(pub String, pub i16, pub i16, pub i16);

impl ModuleVersion {
  pub fn new(module_name: &str, major: i16, minor: i16, patch: i16) -> ModuleVersion {
    ModuleVersion(module_name.into(), major, minor, patch)
  }

  pub fn as_version(&self) -> Version {
    Version(self.1, self.2, self.3)
  }
}

#[derive(Debug)]
pub struct Version(pub i16, pub i16, pub i16);

impl std::cmp::PartialEq for Version {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0 && self.1 == other.1 && self.2 == other.2
  }
}

impl std::cmp::PartialOrd for Version {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    if self == other {
      Some(Ordering::Equal)
    } else if self.0 < other.0 || self.1 < other.1 || self.2 < other.2 {
      Some(Ordering::Less)
    } else {
      Some(Ordering::Greater)
    }
  }
}

#[cfg(test)]
mod module_version {
  pub(self) use super::Version;

  #[test]
  fn version_nltngt() {
    let result1 = Version(1, 1, 1) < Version(1, 1, 1);
    let result2 = Version(1, 1, 1) > Version(1, 1, 1);

    assert_eq!(result1 || result2, false);
  }

  #[cfg(test)]
  mod major {
    use super::Version;

    #[test]
    fn version_major_eq() {
      let result = Version(1, 1, 1) == Version(1, 1, 1);

      assert_eq!(result, true);
    }

    #[test]
    fn version_major_lt() {
      let result = Version(0, 1, 1) < Version(1, 1, 1);

      assert_eq!(result, true);
    }

    #[test]
    fn version_major_gt() {
      let result = Version(2, 1, 1) > Version(1, 1, 1);

      assert_eq!(result, true);
    }
  }

  #[cfg(test)]
  mod minor {
    use super::Version;

    #[test]
    fn version_minor_eq() {
      let result = Version(1, 1, 1) == Version(1, 1, 1);

      assert_eq!(result, true);
    }

    #[test]
    fn version_minor_lt() {
      let result = Version(1, 0, 1) < Version(1, 1, 1);

      assert_eq!(result, true);
    }

    #[test]
    fn version_minor_gt() {
      let result = Version(1, 2, 1) > Version(1, 1, 1);

      assert_eq!(result, true);
    }
  }

  #[cfg(test)]
  mod patch {
    use super::Version;

    #[test]
    fn version_patch_eq() {
      let result = Version(1, 1, 1) == Version(1, 1, 1);

      assert_eq!(result, true);
    }

    #[test]
    fn version_patch_lt() {
      let result = Version(1, 1, 0) < Version(1, 1, 1);

      assert_eq!(result, true);
    }

    #[test]
    fn version_patch_gt() {
      let result = Version(1, 1, 2) > Version(1, 1, 1);

      assert_eq!(result, true);
    }
  }
}
