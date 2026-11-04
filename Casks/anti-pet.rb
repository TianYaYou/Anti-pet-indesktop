cask "anti-pet" do
  version "0.5.1"
  sha256 "d73c767bcda2122e15c15db3511cb90793d09d156d502f0f9e749f16f2f7a09e"

  url "https://github.com/TianYaYou/Anti-pet-indesktop/releases/download/v#{version}/Anti-Pet-v#{version}-macOS-arm64.dmg"
  name "Anti-Pet"
  name "大肥猫桌宠挂件"
  desc "Cross-platform lightweight desktop pet companion for Antigravity"
  homepage "https://github.com/TianYaYou/Anti-pet-indesktop"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :big_sur"
  depends_on arch: :arm64

  app "Anti-Pet.app"

  zap trash: [
    "~/.antigravity-pet-settings.json",
    "~/.antigravity-pet-state.json",
    "~/.antigravity-pet-ledger.json",
  ]
end
