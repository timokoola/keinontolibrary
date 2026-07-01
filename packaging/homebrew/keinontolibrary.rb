class Keinontolibrary < Formula
  desc "Decline Finnish nouns (Kotus classes 1-51, Voikko-verified)"
  homepage "https://keinonto.com"
  url "https://github.com/timokoola/keinontolibrary/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "595027d3d48f15a1a79904095a7b7d15607e01078e0c1231a2075255e253ae69"
  license "MIT"
  head "https://github.com/timokoola/keinontolibrary.git", branch: "main"

  depends_on "rust" => :build

  # The data-backed artifact cannot be rebuilt without the private reference corpus, so it
  # ships as a release asset (cleared for redistribution — see LICENSING.md). Upload
  # keinontolibrary.bin + overlay.jsonl to the v0.1.0 release, then fill the sha256s below
  # (packaging/homebrew/update-shas.sh does this).
  resource "artifact" do
    url "https://github.com/timokoola/keinontolibrary/releases/download/v0.1.0/keinontolibrary.bin"
    sha256 "REPLACE_WITH_ARTIFACT_SHA256"
  end

  resource "overlay" do
    url "https://github.com/timokoola/keinontolibrary/releases/download/v0.1.0/overlay.jsonl"
    sha256 "REPLACE_WITH_OVERLAY_SHA256"
  end

  def install
    system "cargo", "install", "--locked", "--root", libexec, "--path", "crates/keinontolibrary-cli"

    # Install the data-backed artifact + overlay, then wrap the binary so it finds them.
    resource("artifact").stage { pkgshare.install "keinontolibrary.bin" }
    resource("overlay").stage { pkgshare.install "overlay.jsonl" }
    (bin/"keinontolibrary").write_env_script libexec/"bin/keinontolibrary",
      KEINONTO_ARTIFACT: pkgshare/"keinontolibrary.bin",
      KEINONTO_OVERLAY:  pkgshare/"overlay.jsonl"
  end

  test do
    assert_equal "hevosissa",
      shell_output("#{bin}/keinontolibrary decline hevonen --number plural --case inessive").strip
    assert_match "talossa",
      shell_output("#{bin}/keinontolibrary decline talo --number singular --case inessive")
  end
end
