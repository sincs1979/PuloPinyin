// Minimal fcitx5 InputMethodEngine that loads the Rust 部落 engine and commits text.
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>
#include <fcitx-utils/textformatflags.h>

#include "buluo_engine.h"

#include <cstdlib>
#include <string>
#include <unistd.h>

class BuluoFcitxEngine : public fcitx::InputMethodEngineV2 {
public:
    explicit BuluoFcitxEngine(fcitx::Instance *instance) : instance_(instance) {
        const auto dict = resolveDictPath();
        const auto support = resolveSupportDir();
        engine_ = buluo_engine_new(
            dict.empty() ? nullptr : dict.c_str(),
            support.empty() ? nullptr : support.c_str());
    }

    ~BuluoFcitxEngine() override {
        if (engine_) {
            buluo_engine_free(engine_);
            engine_ = nullptr;
        }
    }

    void activate(const fcitx::InputMethodEntry &, fcitx::InputContextEvent &) override {}

    void deactivate(const fcitx::InputMethodEntry &, fcitx::InputContextEvent &event) override {
        resetKey(event.inputContext());
    }

    void reset(const fcitx::InputMethodEntry &, fcitx::InputContextEvent &event) override {
        resetKey(event.inputContext());
    }

    void keyEvent(const fcitx::InputMethodEntry &, fcitx::KeyEvent &keyEvent) override {
        if (keyEvent.isRelease() || !engine_) {
            return;
        }
        int kind = 0;
        uint32_t ch = 0;
        if (!mapKey(keyEvent, &kind, &ch)) {
            return;
        }
        BuluoOutput out{};
        buluo_handle_key(engine_, kind, ch, &out);
        auto *ic = keyEvent.inputContext();
        if (out.consumed) {
            keyEvent.filterAndAccept();
        }
        if (out.commit && out.commit[0]) {
            ic->commitString(out.commit);
        }
        applyPanel(ic, &out);
        buluo_output_free(&out);
    }

private:
    void resetKey(fcitx::InputContext *ic) {
        if (!engine_ || !ic) {
            return;
        }
        BuluoOutput out{};
        buluo_handle_key(engine_, BULUO_KEY_ESCAPE, 0, &out);
        buluo_output_free(&out);
        ic->inputPanel().reset();
        ic->updatePreedit();
        ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
    }

    static bool mapKey(const fcitx::KeyEvent &event, int *kind, uint32_t *ch) {
        const auto &key = event.key();
        const auto sym = key.sym();
        if (sym >= FcitxKey_a && sym <= FcitxKey_z) {
            *kind = BULUO_KEY_CHAR;
            *ch = static_cast<uint32_t>(sym);
            return true;
        }
        if (sym >= FcitxKey_A && sym <= FcitxKey_Z) {
            *kind = BULUO_KEY_CHAR;
            *ch = static_cast<uint32_t>(sym);
            return true;
        }
        if (sym == FcitxKey_space) {
            *kind = BULUO_KEY_SPACE;
            return true;
        }
        if (sym == FcitxKey_BackSpace) {
            *kind = BULUO_KEY_BACKSPACE;
            return true;
        }
        if (sym == FcitxKey_Return || sym == FcitxKey_KP_Enter) {
            *kind = BULUO_KEY_ENTER;
            return true;
        }
        if (sym == FcitxKey_Escape) {
            *kind = BULUO_KEY_ESCAPE;
            return true;
        }
        if (sym == FcitxKey_minus || sym == FcitxKey_comma) {
            *kind = BULUO_KEY_PAGE_PREV;
            return true;
        }
        if (sym == FcitxKey_equal || sym == FcitxKey_period) {
            *kind = BULUO_KEY_PAGE_NEXT;
            return true;
        }
        if (sym == FcitxKey_Shift_L || sym == FcitxKey_Shift_R) {
            *kind = BULUO_KEY_TOGGLE_ASCII;
            return true;
        }
        if (sym == FcitxKey_apostrophe) {
            *kind = BULUO_KEY_SEPARATOR;
            return true;
        }
        if (sym >= FcitxKey_0 && sym <= FcitxKey_9) {
            *kind = BULUO_KEY_DIGIT;
            *ch = static_cast<uint32_t>(sym - FcitxKey_0);
            return true;
        }
        if (sym >= FcitxKey_KP_0 && sym <= FcitxKey_KP_9) {
            *kind = BULUO_KEY_DIGIT;
            *ch = static_cast<uint32_t>(sym - FcitxKey_KP_0);
            return true;
        }
        if (sym > 32 && sym < 127 && !key.hasModifier()) {
            *kind = BULUO_KEY_PUNCT;
            *ch = static_cast<uint32_t>(sym);
            return true;
        }
        return false;
    }

    static void applyPanel(fcitx::InputContext *ic, const BuluoOutput *out) {
        auto &panel = ic->inputPanel();
        panel.reset();
        if (out->preedit && out->preedit[0]) {
            fcitx::Text preedit;
            preedit.append(out->preedit, fcitx::TextFormatFlag::Underline);
            panel.setClientPreedit(preedit);
            panel.setPreedit(preedit);
        }
        if (out->candidate_count > 0 && out->candidates) {
            fcitx::Text aux;
            for (int i = 0; i < out->candidate_count; ++i) {
                if (!out->candidates[i]) {
                    continue;
                }
                if (i) {
                    aux.append("  ");
                }
                aux.append(std::to_string(i + 1) + "." + out->candidates[i]);
            }
            panel.setAuxDown(aux);
        }
        ic->updatePreedit();
        ic->updateUserInterface(fcitx::UserInterfaceComponent::InputPanel);
    }

    static std::string resolveSupportDir() {
        if (const char *xdg = std::getenv("XDG_DATA_HOME")) {
            return std::string(xdg) + "/buluo-ime";
        }
        if (const char *home = std::getenv("HOME")) {
            return std::string(home) + "/.local/share/buluo-ime";
        }
        return {};
    }

    static std::string resolveDictPath() {
        std::string candidates[5];
        size_t n = 0;
        const auto support = resolveSupportDir();
        if (!support.empty()) {
            candidates[n++] = support + "/system.dict";
        }
        if (const char *home = std::getenv("HOME")) {
            candidates[n++] = std::string(home) + "/.local/share/fcitx5/buluo/system.dict";
        }
        candidates[n++] = "/usr/share/buluo-ime/system.dict";
        candidates[n++] = "/usr/local/share/buluo-ime/system.dict";
        for (size_t i = 0; i < n; ++i) {
            if (access(candidates[i].c_str(), R_OK) == 0) {
                return candidates[i];
            }
        }
        return {};
    }

    fcitx::Instance *instance_;
    BuluoEngine *engine_ = nullptr;
};

class BuluoFactory : public fcitx::AddonFactory {
public:
    fcitx::AddonInstance *create(fcitx::AddonManager *manager) override {
        return new BuluoFcitxEngine(manager->instance());
    }
};

FCITX_ADDON_FACTORY_V2(buluo, BuluoFactory);
